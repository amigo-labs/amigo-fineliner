# Fineliner

A general-purpose image editor — a spiritual successor to paint.net — focused on
simplicity, speed, and cross-platform reach. One Rust core and one Svelte UI ship
as a PWA today, with a native Tauri shell planned (see the milestone plan in
[CLAUDE.md](CLAUDE.md) §4).

- **What to build:** [`docs/specs/fineliner.md`](docs/specs/fineliner.md) (the spec)
- **How to build it:** [`CLAUDE.md`](CLAUDE.md) (workflow, conventions, milestones)
- **Current state / next task:** [`STATUS.md`](STATUS.md)

## Prerequisites

- **Rust** (stable) with the WASM target: `rustup target add wasm32-unknown-unknown`
- **wasm-pack**: `cargo install wasm-pack` (or `npm install -g wasm-pack`)
- **Node.js** ≥ 20 and **pnpm**

## Run the app

```sh
cd ui
pnpm install
pnpm dev        # builds the WASM core automatically, then starts Vite
```

Every `dev`/`build`/`check` script first runs `pnpm run wasm`, which compiles
`crates/fineliner-wasm` with wasm-pack into the gitignored `ui/src/lib/wasm/pkg/`.
A fresh clone needs nothing beyond the prerequisites above — the first script run
generates the package.

## Verify

```sh
# Rust workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# UI (TypeScript / Svelte)
cd ui
pnpm check      # wasm build + svelte-check (also aliased as `pnpm lint`)
pnpm build      # production bundle
```

UI unit/e2e tests are a Phase 3 item (CLAUDE.md §7.5); there is no `pnpm test` yet.

## Repository layout

```
crates/fineliner-core/    Pure image-editing logic — no I/O, no platform deps
crates/fineliner-wasm/    wasm-bindgen bindings (cdylib) consumed by the UI
ui/                       Svelte 5 + Vite + Tailwind 4 frontend
docs/specs/fineliner.md   Design specification (source of truth for behavior)
CLAUDE.md                 Implementation contract: workflow, conventions, ADR log
STATUS.md                 Session state: completed milestones, next task
```

## License

MIT OR Apache-2.0. The bundled Liberation Sans font is licensed under the
SIL Open Font License 1.1 (`ui/public/fonts/LiberationSans-LICENSE.txt`).
