# Build
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app
COPY Cargo.* .

RUN mkdir src && echo "fn main() {}" > ./src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry <<EOF
  set -e
  # update timestamps to force a new build
  touch /app/src/main.rs
  cargo build --release
EOF

# Runtime
FROM debian:bookworm-slim

COPY --from=builder /app/target/release/url-shortener .

CMD ["./url-shortener"]

