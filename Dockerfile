# Builder Stage
FROM rustlang/rust:nightly as builder

WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY . .
RUN cargo build --release

# Runtime Stage with OpenSSL 3
FROM debian:bookworm-slim

# Install just the runtime OpenSSL libraries
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/api /app/api
COPY migrations ./migrations

CMD ["./api"]
