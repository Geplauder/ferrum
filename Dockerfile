# -----------------
# Create recipe
# -----------------
FROM rust as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -----------------
# Restore dependencies from recipe
# -----------------
FROM rust as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# -----------------
# Build project
# -----------------
FROM rust as builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
ENV SQLX_OFFLINE true
RUN cargo build --release --bin ferrum

# -----------------
# Runtime
# -----------------
FROM debian:buster-slim
RUN apt-get update && apt-get install -yy libssl1.1 ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/ferrum ferrum
COPY settings/base.json settings/base.json
ENV APP_ENV production
ENTRYPOINT ["./ferrum"]