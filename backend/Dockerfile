# syntax=docker/dockerfile:1.7

FROM rust:1.91.1 AS rust-base
WORKDIR /app
RUN cargo install cargo-chef

# Planner
FROM rust-base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Cached Builder
FROM rust-base AS builder
WORKDIR /app

# Multi-platform support: set RUSTFLAGS only for amd64
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "amd64" ]; then \
       export RUSTFLAGS="-C target-cpu=x86-64-v2"; \
    fi

RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y build-essential cmake pkg-config protobuf-compiler

# Use build secret for GH token
# Make sure Git knows to use the token from secrets
# RUN --mount=type=secret,id=gh_token \
#   bash -c 'TOKEN="$(cat /run/secrets/gh_token 2>/dev/null || true)"; \
#   [ -n "$TOKEN" ] || { echo "GH_TOKEN MISSING OR EMPTY" >&2; exit 1; }; \
#   git config --global url."https://${TOKEN}@github.com/".insteadOf "https://github.com/"'

# Cache / Check for Cargo Changes (the magic)
RUN cargo chef cook --release --bin backend --recipe-path=recipe.json

# Build
COPY . .
RUN cargo build --release --bin backend

# Final Container
FROM gcr.io/distroless/cc-debian13
WORKDIR /app
COPY --from=builder /app/target/release/backend /bin/backend
EXPOSE 3030
ENTRYPOINT ["/bin/backend"]
CMD ["mono"]
