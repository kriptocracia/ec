FROM rust:1.87 AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ proto/
COPY src/ src/
COPY migrations/ migrations/
COPY rules/ rules/

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libsqlite3-0 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/ec /usr/local/bin/ec
COPY migrations/ /app/migrations/
COPY rules/ /app/rules/
COPY ec.toml /app/ec.toml

WORKDIR /app

EXPOSE 50051

CMD ["ec"]
