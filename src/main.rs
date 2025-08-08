mod constant;
mod database;
mod schema;

use crate::constant::*;
use crate::database::*;
use anyhow::{Context, Result, bail};
use dotenvy::dotenv;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::Value;
use std::env::var;

fn main() -> Result<()> {
    dotenv().ok();

    // Connection
    let postgres_url =
        var("POSTGRES_URL").unwrap_or_else(|_| "postgres://localhost:5432/postgres".to_string());
    let vault_addr = var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());

    // Database names
    let auth_db = read_value_with_default("AUTH_DB", "ytx_auth")?;
    let main_db = read_value_with_default("MAIN_DB", "ytx_main")?;
    let main_workspace = read_value_with_default("MAIN_WORKSPACE", "ytx_workspace")?;

    // Roles
    let postgres_role = read_value_with_default("POSTGRES_ROLE", "postgres")?;
    let auth_readwrite_role = read_value_with_default("AUTH_READWRITE_ROLE", "ytx_auth_readwrite")?;
    let main_readwrite_role = read_value_with_default("MAIN_READWRITE_ROLE", "ytx_main_readwrite")?;
    let main_readonly_role = read_value_with_default("MAIN_READONLY_ROLE", "ytx_main_readonly")?;

    // Passwords (can be overridden by Vault)
    let mut postgres_password = var("POSTGRES_PASSWORD").unwrap_or_default();
    let mut auth_readwrite_password = var("AUTH_READWRITE_PASSWORD").unwrap_or_default();
    let mut main_readwrite_password = var("MAIN_READWRITE_PASSWORD").unwrap_or_default();
    let mut main_readonly_password = var("MAIN_READONLY_PASSWORD").unwrap_or_default();

    if let Ok(postgres_token) = var("POSTGRES_TOKEN") {
        if !postgres_token.is_empty() {
            let pg_data = read_vault_data(&vault_addr, &postgres_token, POSTGRES_SECRET_PATH)
                .context("Failed to read PostgreSQL superuser password from Vault")?;
            postgres_password = get_vault_password(&pg_data, &postgres_role)?;

            let ytx_data = read_vault_data(&vault_addr, &postgres_token, YTX_SECRET_PATH)
                .context("Failed to read YTX role passwords from Vault")?;
            auth_readwrite_password = get_vault_password(&ytx_data, &auth_readwrite_role)?;
            main_readonly_password = get_vault_password(&ytx_data, &main_readonly_role)?;
            main_readwrite_password = get_vault_password(&ytx_data, &main_readwrite_role)?;
        }
    }

    let full_postgres_url = build_url(&postgres_url, &postgres_role, &postgres_password)?;
    let mut postgres_client = postgres::Client::connect(&full_postgres_url, postgres::NoTls)
        .context("Failed to connect to PostgreSQL server")?;

    create_database(&mut postgres_client, &auth_db)?;
    create_database(&mut postgres_client, &main_db)?;

    create_role(
        &mut postgres_client,
        &auth_readwrite_role,
        &auth_readwrite_password,
    )?;

    create_role(
        &mut postgres_client,
        &main_readonly_role,
        &main_readonly_password,
    )?;

    create_role(
        &mut postgres_client,
        &main_readwrite_role,
        &main_readwrite_password,
    )?;

    let auth_url = replace_postgres_url(&full_postgres_url, &auth_db)?;
    let mut auth_client = postgres::Client::connect(&auth_url, postgres::NoTls)?;

    initialize_auth_database(&mut auth_client)?;
    insert_workspace_database(&mut auth_client, &main_workspace, &main_db)?;

    let main_url = replace_postgres_url(&full_postgres_url, &main_db)?;
    let mut main_client = postgres::Client::connect(&main_url, postgres::NoTls)?;
    initialize_main_database(&mut main_client)?;

    grant_readonly_permission(
        &mut postgres_client,
        &mut main_client,
        &main_db,
        &main_readonly_role,
    )?;

    grant_readwrite_permission(
        &mut postgres_client,
        &mut main_client,
        &main_db,
        &main_readwrite_role,
    )?;

    grant_readwrite_permission(
        &mut postgres_client,
        &mut auth_client,
        &auth_db,
        &auth_readwrite_role,
    )?;

    Ok(())
}

fn read_vault_data(vault_addr: &str, token: &str, secret_path: &str) -> Result<Value> {
    let url = format!("{}/v1/{}", vault_addr.trim_end_matches('/'), secret_path);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    let resp = Client::new().get(&url).headers(headers).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP error {}", resp.status());
    }

    let json: Value = resp.json()?;
    Ok(json["data"]["data"].clone())
}

fn get_vault_password(data: &serde_json::Value, key: &str) -> Result<String> {
    data.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Vault key '{}' not found or not a string", key))
}

fn read_value_with_default(key: &str, default: &str) -> Result<String> {
    let val = var(key).unwrap_or(default.to_string());

    if val.is_empty() {
        bail!("Value for '{}' cannot be empty", key);
    }

    if val.len() > 63 {
        bail!("Value for '{}' cannot be longer than 63 characters", key);
    }

    let mut chars = val.chars();
    let first = chars.next().unwrap();

    if !first.is_ascii_lowercase() {
        bail!("Value for '{}' must start with a lowercase letter", key);
    }

    if !val
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!(
            "Value for '{}' can only contain lowercase letters, digits, and underscore",
            key
        );
    }

    Ok(val)
}
