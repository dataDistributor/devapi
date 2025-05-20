# Builder stage
FROM rustlang/rust:nightly as builder
WORKDIR /usr/src/app

# Install required dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Copy all files into container
COPY . .

# Build in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy built binary from builder
COPY --from=builder /usr/src/app/target/release/api /app/api

# Copy migrations folder
COPY migrations ./migrations

# Runtime command
CMD ["./api"]
