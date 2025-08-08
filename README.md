# YTX InitDB

## Overview

YTX InitDB is the database initialization tool for the YTX ERP system. It automates the creation of PostgreSQL databases and roles, configures database permissions, and initializes necessary schema objects.
It supports securely fetching database passwords dynamically from Vault to ensure secret management best practices.

## Features

- Automatically create PostgreSQL databases (e.g., `ytx_auth`, `ytx_main`)
- Automatically create PostgreSQL roles (`ytx_admin`, `ytx_readonly`, `ytx_readwrite`, etc.)
- Supports two password sourcing methods: environment variables or Vault secrets
- Initializes database schema and essential data
- Configures granular role permissions for secure data access
- Detailed error handling and logging for easier troubleshooting

## Technology Stack

- Developed in Rust
- Uses `postgres` crate for PostgreSQL connectivity and queries
- Uses `reqwest` crate to interact with Vault HTTP API for secret retrieval
- Uses `dotenvy` crate for environment variable loading
- Vault secret data formatted as JSON for flexible key-value storage

## Password Sourcing Priority

- If a valid `POSTGRES_TOKEN` (Vault token) is provided, **all passwords will be fetched dynamically from Vault**, overriding any password values set in environment variables.
- If no `POSTGRES_TOKEN` is provided or the token is invalid, the system will **fall back to using the passwords directly set in environment variables**.
- This design offers flexibility to use Vault for secure centralized management in production, or environment variables for simple local/testing scenarios.

## Configuration

- The `.env` file holds fallback passwords and basic configuration parameters.
- Vault address and authentication tokens are provided via environment variables.
- Database names and PostgreSQL role names can be customized to suit your environment.
- Default PostgreSQL roles are: `ytx_admin`, `ytx_readonly`, `ytx_readwrite`.
- Vault configuration details:
  - PostgreSQL superuser password is stored at: `secret/data/postgres/postgres`.
  - Passwords for YTX roles are stored under: `secret/data/postgres/ytx`, with keys matching the role names.
  - Vault uses the KV version 2 secrets engine, ensure your Vault instance is configured accordingly.
  - Password keys stored in Vault **must exactly match** the PostgreSQL role names configured in your environment.
- Each workspace should be uniquely associated with one main database to ensure data isolation and integrity.

## Usage Steps

1. Prepare PostgreSQL server instance
2. Set up Vault service, upload passwords and policies accordingly
3. Configure `.env` file or environment variables with Vault address, tokens, and DB connection info
4. Run `ytx-initdb` to initialize databases, roles, permissions, and schemas automatically
5. Verify successful creation of databases and roles with correct permissions

## Security Considerations

- All passwords are centrally managed in Vault to avoid hardcoding secrets.
- Vault tokens can be short-lived and support renewal for improved security.
- Principle of least privilege is followed for role permissions.
- **Ensure that the `.env` file containing fallback passwords and tokens has strict file permissions (e.g., `chmod 600 .env`) to prevent unauthorized access and potential secret leakage.**
