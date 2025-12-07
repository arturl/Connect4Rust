# Builder for Rust and frontend
FROM rust:1.81-slim-bookworm AS builder
WORKDIR /app

# Install node for frontend build (force https sources)
RUN printf "deb https://deb.debian.org/debian bookworm main\n\
deb https://deb.debian.org/debian bookworm-updates main\n\
deb https://security.debian.org/debian-security bookworm-security main\n" > /etc/apt/sources.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates nodejs npm pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY connect4/Cargo.toml connect4/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
RUN mkdir -p connect4/src server/src
RUN echo "fn main() {}" > server/src/main.rs && echo "// stub" > connect4/src/lib.rs
RUN cargo build -p server --release || true

# Real sources
COPY connect4 ./connect4
COPY server ./server
COPY web ./web
COPY README.md .

# Build frontend
WORKDIR /app/web
RUN npm install && npm run build

# Build backend with vendored dist
WORKDIR /app
# Remove dummy binaries to force recompilation with real source
RUN rm -rf target/release/.fingerprint/server-* target/release/server target/release/deps/server-* \
           target/release/.fingerprint/connect4-* target/release/libconnect4.* target/release/deps/libconnect4-*
RUN cargo build -p server --release

# Runtime image
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/web/dist /app/web/dist
ENV RUST_LOG=info
EXPOSE 3000
CMD ["./server"]
