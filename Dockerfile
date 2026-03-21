FROM rust:1.94 AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ proto/
COPY src/ src/
COPY migrations/ migrations/
COPY rules/ rules/

RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 && rm -rf /var/lib/apt/lists/*

RUN groupadd -r ecuser && useradd -r -g ecuser -d /app ecuser

COPY --from=builder /build/target/release/ec /usr/local/bin/ec
COPY migrations/ /app/migrations/
COPY rules/ /app/rules/
COPY ec.toml /app/ec.toml

RUN chown -R ecuser:ecuser /app

WORKDIR /app

USER ecuser

EXPOSE 50051

CMD ["ec"]
