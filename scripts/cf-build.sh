#!/usr/bin/env bash
#
# Cloudflare Workers Builds — build the Fineliner PWA into ui/dist.
#
# Cloudflare's build image ships Node but not the Rust/wasm-pack toolchain the
# WASM core needs, so this script installs what's missing and then runs the
# normal `pnpm build`. Configure the Cloudflare Workers project (Settings →
# Build) as:
#
#   Root directory:  /            (repo root)
#   Build command:   bash scripts/cf-build.sh
#   Deploy command:  npx wrangler deploy      (the default; uses wrangler.jsonc)
#
# wrangler.jsonc serves ./ui/dist as a single-page app.
set -euo pipefail

# --- Rust toolchain -------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$HOME/.cargo/bin:$PATH"

rustup target add wasm32-unknown-unknown

# --- wasm-pack (prebuilt if possible, else from source) -------------------
if ! command -v wasm-pack >/dev/null 2>&1; then
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh || true
fi
if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack --locked
fi

# --- pnpm via corepack (Node is preinstalled) -----------------------------
corepack enable
corepack prepare pnpm@10 --activate

# --- Build ----------------------------------------------------------------
cd ui
pnpm install --frozen-lockfile
pnpm build

echo "Built PWA into ui/dist"
