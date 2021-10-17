# Ferrum
[![codecov](https://codecov.io/gh/Geplauder/ferrum/branch/main/graph/badge.svg?token=ns58EurBKz)](https://codecov.io/gh/Geplauder/ferrum)
![](../../actions/workflows/rust.yml/badge.svg)

Ferrum is the backend for Geplauder, built with actix-web and sqlx.

For more information on Geplauder, [click here](../../../).

## Architecture

Ferrum consists of two separate services:
- REST Api: Handles initial data fetching for frontend clients and processes actions from the client.
- WebSocket Server: Keep connected clients from the frontend up-to-date by providing them with appropriate updates.

<img src="https://cdn.cryma.de/9e41b05a-d5ff-4df1-b854-87eef15a9131.png" width="75%" />

## Usage

This project supplies a `docker-compose.yml` file for easy development.

Start Postgres and RabbitMQ:
```bash
docker compose up -d
```

The preferred way to configure your database is `sqlx-cli`:
```bash
cargo install sqlx-cli # Install sqlx-cli
cd ferrum-db
sqlx database create   # Create the database specified in '.env'
sqlx migrate run       # Run migrations
```

As this project contains two services, we need to start them both for proper usage.

Start the REST api:
```bash
cargo run
```

Start the websocket server:
```bash
cd ferrum-websocket
cargo run
```

As this project uses `bunyan` with a JSON formatter for logging (tracing), it's recommended to run it in combination with `bunyan`:
```bash
npm install -g bunyan # Globally install bunyan
cargo run | bunyan    # Start the project and pipe the logs into bunyan
```

## Contributing

Ferrum uses `rustfmt` for formatting and `clippy` for linting. Make sure to run them before committing:
```bash
cargo fmt
cargo clippy
```

When using `sqlx` compile-time checked queries, make sure to update the metadata before committing:
```bash
cd ferrum-db
cargo sqlx prepare # Update sqlx metadata
```

## Disclaimer

This project is under heavy development and not suitable for production. It may contain severe security vulnerabilities.

## License
[GNU General Public License v3 (GPL-3)](./LICENSE) unless otherwise stated.
