# Repository Guidelines

## Project Structure & Module Organization
- Root: Rust workspace (`Cargo.toml`) with `sealbox-server` (Axum API, SQLite via SQLx) and `sealbox-cli` (CLI). Frontend lives in `sealbox-web` (Vite + React + TypeScript). Docs in `docs/`.
- Server: `sealbox-server/src/**` (API, crypto, repo). Entrypoint: `sealbox-server/src/main.rs`.
- CLI: `sealbox-cli/src/**` (commands, config). Entrypoint: `sealbox-cli/src/main.rs`.
- Web: `sealbox-web/src/**` (routes, components, stores). Assets in `sealbox-web/src/assets/`.

## Build, Test, and Development Commands
- Rust build: `cargo build --workspace` (add `--release` for optimized binaries).
- Rust tests: `cargo test --workspace`.
- Run server: `cargo run -p sealbox-server`.
- Run CLI: `cargo run -p sealbox-cli -- --help` (append subcommands, e.g., `... client register`, `... secret set`).
- Lint/format (Rust): `cargo fmt --all` and `cargo clippy --workspace -- -D warnings`.
- Web dev: `pnpm -C sealbox-web install && pnpm -C sealbox-web dev`.
- Web build: `pnpm -C sealbox-web build`.
- Docker (optional): `docker build -t sealbox .` using the root `Dockerfile`.

## Coding Style & Naming Conventions
- Rust: use `rustfmt` defaults; fix clippy warnings before PRs. Files/modules `snake_case`; types/enums `PascalCase`; functions `snake_case`.
- Web: Prettier via lint-staged; 2-space indent. Files `kebab-case`/lowercase (e.g., `routes/login.tsx`); React components export `PascalCase` identifiers.
- Keep functions small and focused; prefer explicit types at public boundaries.
- Remove unused code instead of adding `#[allow(dead_code)]` or similar suppressions.

## Documentation Guidelines
- English only. Do not include non-English text in docs.
- Keep docs in `docs/` and `README.md` in sync with code changes (CLI flags, endpoints, and flows like enrollment).
- When behavior/config changes, update relevant docs sections in the same PR.

## Testing Guidelines
- Rust: place unit tests inline with `mod tests { ... }`; integration tests in `sealbox-*/tests/`. Run `cargo test --workspace` locally. Coverage is not enforced; add tests for new behavior and bug fixes.
- Web: no formal test runner configured. If adding tests, prefer Vitest and colocate near components.

## Commit & Pull Request Guidelines
- Commit style: conventional commits (e.g., `feat: ...`, `fix: ...`, `refactor: ...`, `style: ...`).
- Before PR: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and `pnpm -C sealbox-web build`.
- PRs must include: clear description, linked issues, screenshots/GIFs for `sealbox-web` UI changes, and docs updates in `docs/` when behavior/config changes.
- Do not commit secrets or local artifacts (`.env`, `sealbox.db*`). Use `.env.example` as reference (see `docs/configuration.md`).
