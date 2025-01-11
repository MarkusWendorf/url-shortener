# Build
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app
COPY Cargo.* .

RUN mkdir src && echo "fn main() {}" > ./src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

# Runtime
FROM debian:bookworm-slim

COPY --from=builder /app/target/release/url-shortener .

CMD ["./url-shortener"]

