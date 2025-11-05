# Feeder Vision

Rust workspace for an offline wildlife feeder image review tool. The goal is to scan a folder of frames, detect animal presence, and (optionally) classify species with open‑set abstention. See `specs/` for product and scenario details.

## Requirements
- Rust toolchain (1.75+ recommended) with `cargo`
- Windows/macOS/Linux

## Quick Start
- Build: `cargo build`
- Run example binary (if present): `cargo run -p feeder_vision` or `cargo run` (workspace root)
- Test: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format check: `cargo fmt --all -- --check`

For CI‑like local checks on Windows: `./scripts/ci.ps1`

## Specs and Scenarios
- Product spec: `specs/product-spec.md`
- Acceptance scenarios: `specs/scenarios.md`

End‑to‑end tests in `tests/e2e_spec.rs` currently reference the scenarios and are marked `#[ignore]` with `todo!()` until the E2E harness is implemented. This keeps the spec traceable without creating false‑green tests.

## Repository Layout
- `src/` — workspace binary entry (temporary placeholder)
- `crates/` — application crates (core logic, GUI)
- `tests/` — integration/E2E test stubs
- `scripts/` — helper scripts for CI and spec coverage checks
- `specs/` — product and scenario documentation

## Development Workflow
1) Align changes with `specs/`
2) Keep scenarios mirrored in tests
3) Run clippy/format/test before pushing

## Notes
- Formatting is enforced in CI via `cargo fmt --all -- --check`.
- Some paths/crate names may still be scaffolding; adjust as implementation lands.

