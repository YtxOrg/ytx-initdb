# YTX InitDB

## Overview

**YTX InitDB** is a database initialization tool for the YTX ERP system. It automates the creation of PostgreSQL databases and roles, configures permissions, and initializes schemas. It supports secure password management by dynamically fetching secrets from Vault or using environment variables for local/testing scenarios.

---

## Features

- Automated creation of PostgreSQL databases (e.g., `ytx_auth`, `ytx_main`)
- Automated creation of roles (`ytx_auth_readwrite`, `ytx_main_readwrite`, `ytx_main_readonly`)
- Two password sourcing methods: Vault secrets (recommended) or environment variables (fallback)
- Schema and essential data initialization
- Granular role permissions for secure data access
- Detailed error handling and logging

---

## Technology Stack

- **Language:** Rust
- **Database:** PostgreSQL (`postgres` crate)
- **Secret Management:** Vault (`reqwest` crate for HTTP API)
- **Config:** `.env` file loaded via `dotenvy`
- **Vault:** KV v2 secrets engine, JSON-formatted secret data

---

## Password Management & Security

- **All required PostgreSQL role passwords must be pre-set** in Vault or `.env` before initialization.
- **Priority:**
  1. If a valid `POSTGRES_TOKEN` (Vault token) is provided, all passwords are fetched from Vault (overriding environment variables).
  2. If no valid token, passwords are read from environment variables.
- **Vault secret paths:**
  - Superuser: `secret/data/postgres/postgres`
  - YTX roles: `secret/data/postgres/ytx`
- **Best Practices:**
  - Never hardcode secrets in code or public files.
  - Restrict `.env` permissions: `chmod 600 .env`
  - Vault tokens should be short-lived and renewable.
  - Principle of least privilege for all roles.

---

## Quick Start

### 1. Run PostgreSQL & Vault with Docker

A preconfigured `docker-compose.yml` is provided for local development/testing.

- **PostgreSQL**: persistent storage, configurable password
- **Vault**: local file storage, UI enabled, port mapping
- **Important:** Always wrap `POSTGRES_PASSWORD` in double quotes (`""`) in Docker Compose.

```bash
docker compose -p ytx up -d
```

---

### 2. Configure Environment & Vault

- Copy the environment template:

  ```shell
  cp env_template.text .env
  ```

- Store PostgreSQL superuser password in Vault:

  ```shell
  vault kv put secret/postgres/postgres postgres=POSTGRES_PASSWORD
  ```

- Generate and store random passwords for YTX roles in Vault:

  ```shell
  vault kv put secret/postgres/ytx \
    ytx_auth_readwrite=$(openssl rand -base64 16) \
    ytx_main_readwrite=$(openssl rand -base64 16) \
    ytx_main_readonly=$(openssl rand -base64 16)
  ```

---

### 3. Initialize Database

```shell
git clone https://github.com/YtxErp/ytx-initdb.git
cd ytx-initdb

cargo run --release
```

---

### 4. Verify

```shell
psql -h localhost -U <postgres_user> -d <database_name>
# Example:
psql -h localhost -U postgres -d ytx_auth
psql -h localhost -U postgres -d ytx_main
```

---

## Configuration Reference

- `.env` holds fallback passwords and config parameters.
- Vault address and tokens are provided via environment variables.
- Database and role names are customizable.
- Each workspace should have a unique main database for data isolation.

---

## Support

If YTX has been helpful to you, I’d be truly grateful for your support. Your encouragement helps me keep improving and creating!

Also may the force be with you.

[<img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" width="160" height="40">](https://buymeacoffee.com/ytx.cash)
