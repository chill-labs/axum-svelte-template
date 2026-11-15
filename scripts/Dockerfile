# =============================================================================
# AXUM + SVELTE 5 TEMPLATE MULTI-STAGE DOCKERFILE
# =============================================================================
#
# Builds the static Svelte 5 CSR bundle (Stage 1) and compiles the Rust Axum
# `server` binary with `cargo-chef` dependency caching (Stages 2-5), packaging
# everything into a minimal `debian:bookworm-slim` runtime image (Stage 6).
#
# CRITICAL: Only explicit build inputs are copied into each stage so host
# `node_modules/`, `.svelte-kit/`, `target/`, and `.env` never leak into layers.

# Stage 1: Build Svelte 5 Static CSR Bundle + .br/.gz Sidecars
FROM node:22-bookworm-slim AS web-builder

ENV PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1

WORKDIR /build/svelte-app

COPY svelte-app/package*.json ./
RUN if [ -f package-lock.json ]; then npm ci; else npm install; fi

COPY style/input.css /build/style/input.css
COPY public /build/public
COPY svelte-app/tsconfig.json svelte-app/vite.config.ts ./
COPY svelte-app/src ./src

RUN npm run build

# Stage 2: Rust Stable Toolchain Base (`cargo-chef`, matching rust-toolchain.toml)
FROM rust:1-bookworm AS rust-base

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall cargo-chef -y || cargo install cargo-chef --locked

WORKDIR /app

# Stage 3: Planner (`recipe.json` across Cargo workspace crates)
FROM rust-base AS planner

COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY xtask xtask

RUN cargo chef prepare --recipe-path recipe.json

# Stage 4: Cacher (Compile third-party dependencies for `server`)
FROM rust-base AS cacher

COPY --from=planner /app/recipe.json recipe.json
COPY .cargo .cargo

RUN cargo chef cook --release --package server --bin server --recipe-path recipe.json

# Stage 5: Builder (Compile `server` release binary)
FROM cacher AS rust-builder

COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY xtask xtask
COPY public public
COPY svelte-app/src/app.html svelte-app/src/app.html

ENV CARGO_BUILD_JOBS=4

RUN cargo build --release -p server --bin server \
    && cp /app/target/release/server /app/server

# Stage 6: Minimal Debian Runtime (`debian:bookworm-slim`)
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates \
    libpq5 \
    curl \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/server /app/server
COPY --from=web-builder /build/svelte-app/build /app/svelte-app/build
COPY public /app/public

ENV SVELTE_SITE_ROOT=svelte-app/build
ENV PUBLIC_ASSETS_ROOT=public
ENV APP__SERVER__HOST="0.0.0.0"
ENV APP__SERVER__PORT=3000
ENV APP__ENV="prod"

EXPOSE 3000

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:${APP__SERVER__PORT:-3000}/api/v1/health || exit 1

CMD ["/app/server"]
