use crate::constant::*;
use crate::schema::*;

use anyhow::{Context, Result, bail};
use postgres::Client;
use url::Url;

pub fn create_database(client: &mut Client, database: &str) -> Result<()> {
    let exists: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)",
            &[&database],
        )
        .context("Failed to check if database exists")?
        .get(0);

    if !exists {
        let create_sql = format!("CREATE DATABASE {}", database);
        client
            .execute(&create_sql, &[])
            .with_context(|| format!("Failed to create database `{}`", database))?;
        println!("Database {} created.", database);
    } else {
        println!("Database {} already exists.", database);
    }

    Ok(())
}

pub fn create_role(client: &mut Client, role: &str, password: &str) -> Result<()> {
    let exists: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1)",
            &[&role],
        )
        .context("Failed to check if role exists")?
        .get(0);

    if !exists {
        let row = client.query_one("SELECT quote_literal($1)", &[&password])?;
        let escaped_password: String = row.get(0);

        let sql = format!(
            "CREATE ROLE {} WITH LOGIN PASSWORD {} NOCREATEDB NOCREATEROLE",
            role, escaped_password
        );

        client
            .execute(&sql, &[])
            .with_context(|| format!("Failed to create role `{}`", role))?;
        println!("Role {} created.", role);
    } else {
        println!("Role {} already exists.", role);
    }

    Ok(())
}

pub fn initialize_main_database(client: &mut Client) -> Result<()> {
    let mut transaction = client.transaction()?;

    let mut sqls = Vec::new();
    sqls.extend([
        ytx_meta(),
        global_config(),
        f_node_table(),
        s_node_table(),
        i_node_table(),
        t_node_table(),
        f_entry_table(),
        s_entry_table(),
        t_entry_table(),
        i_entry_table(),
    ]);

    for section in SECTIONS {
        sqls.push(path_table(section));
        sqls.push(insert_global_config(section));
    }

    for section in [SALE, PURCHASE] {
        sqls.push(o_node_table(section));
        sqls.push(o_entry_table(section));
        sqls.push(o_settlement_table(section));
    }

    sqls.push(insert_meta());

    for sql in sqls {
        if let Err(e) = transaction.execute(&sql, &[]) {
            let _ = transaction.rollback();
            bail!("Failed to execute SQL `{sql}`: {e}");
        }
    }

    transaction.commit()?;
    Ok(())
}

pub fn initialize_auth_database(client: &mut Client) -> Result<()> {
    let mut transaction = client.transaction()?;

    let mut sqls = Vec::new();
    sqls.extend([ytx_user(), ytx_role_workspace(), ytx_workspace_database()]);

    for sql in sqls {
        if let Err(e) = transaction.execute(&sql, &[]) {
            let _ = transaction.rollback();
            bail!("Failed to execute SQL `{sql}`: {e}");
        }
    }

    transaction.commit()?;
    Ok(())
}

pub fn grant_readonly_permission(
    postgres_client: &mut Client,
    client: &mut Client,
    database: &str,
    role: &str,
) -> Result<()> {
    postgres_client.execute(
        &format!("GRANT CONNECT ON DATABASE {} TO {}", database, role),
        &[],
    )?;

    client.execute(&format!("GRANT USAGE ON SCHEMA public TO {}", role), &[])?;

    client.execute(
        &format!("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {}", role),
        &[],
    )?;

    client.execute(
        &format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO {}",
            role
        ),
        &[],
    )?;

    Ok(())
}

pub fn grant_readwrite_permission(
    postgres_client: &mut Client,
    client: &mut Client,
    database: &str,
    role: &str,
) -> Result<()> {
    postgres_client.execute(
        &format!("GRANT CONNECT ON DATABASE {} TO {}", database, role),
        &[],
    )?;

    client.execute(&format!("GRANT USAGE ON SCHEMA public TO {}", role), &[])?;

    client.execute(
        &format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}",
            role
        ),
        &[],
    )?;

    client.execute(
        &format!(
            "GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO {}",
            role
        ),
        &[],
    )?;

    client.execute(
        &format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}",
            role
        ),
        &[],
    )?;

    client.execute(
        &format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO {}",
            role
        ),
        &[],
    )?;

    Ok(())
}

pub fn replace_postgres_url(postgres_url: &str, new_db: &str) -> Result<String> {
    let mut url = Url::parse(postgres_url)?;
    url.set_path(&format!("/{}", new_db));
    Ok(url.to_string())
}

pub fn build_url(base_url: &str, user: &str, password: &str) -> Result<String> {
    let mut url = Url::parse(base_url)?;
    url.set_username(user)
        .map_err(|()| anyhow::anyhow!("Invalid username"))?;
    url.set_password(Some(password))
        .map_err(|()| anyhow::anyhow!("Invalid password"))?;
    Ok(url.into())
}

pub fn insert_workspace_database(
    client: &mut Client,
    workspace: &str,
    database: &str,
) -> Result<()> {
    let row = client.query_opt(
        "SELECT database FROM ytx_workspace_database WHERE workspace = $1",
        &[&workspace],
    )?;

    if let Some(row) = row {
        let existing_db: String = row.get(0);
        if existing_db == database {
            return Ok(());
        } else {
            bail!(
                "Workspace '{}' is already linked to a different database '{}', please check your .env configuration.",
                workspace,
                existing_db
            );
        }
    }

    client.execute(
        r#"
        INSERT INTO ytx_workspace_database (workspace, database)
        VALUES ($1, $2);
    "#,
        &[&workspace, &database],
    )?;
    println!(
        "Workspace '{}' linked to database '{}'",
        workspace, database
    );

    Ok(())
}
