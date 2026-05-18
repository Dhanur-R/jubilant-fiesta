# Stage 1: Generate CSS
FROM node:26-slim AS css-builder
WORKDIR /app
COPY tailwind.config.js ./
COPY styles ./styles
COPY templates ./templates
RUN npx --yes tailwindcss@3 -i styles/input.css -o static/styles.css --minify

# Stage 2: Build Rust
FROM rust:1-bookworm AS builder
WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build the actual application
COPY src ./src
COPY templates ./templates
COPY static ./static
RUN touch src/main.rs && cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/linkr /usr/local/bin/linkr
COPY --from=css-builder /app/static /app/static
WORKDIR /app
CMD ["linkr"]
