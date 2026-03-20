# Rad Roots - Code Directives

- this repo is a standalone community-edition OSS app and must remain cloneable and usable from this repo root
- do not treat the outer workspace as the authoritative runtime or release root for this repo
- during the current integrated development phase, temporary local path dependencies are allowed only to the public `radroots` crate family and approved developer-controlled vendor repos
- do not add temporary local path dependencies from this repo to archived code, `refs/*`, or private platform crates without explicit approval
- validate from this repo root with `cargo metadata --format-version 1 --no-deps` and `cargo check`

## Rust Code Directives
- Toolchain: Rust 1.92, edition 2024; use workspace versions from the root Cargo.toml.
- Portability: preserve no_std patterns; gate std usage with cfg(feature = "std") and use alloc when needed.
- Safety: avoid unsafe; prefer safe, explicit APIs. Add #![forbid(unsafe_code)] on new crates/modules.
- Public API: keep Radroots* prefix; avoid hidden panics; return Result/Option for fallible ops; use precise error enums (thiserror where appropriate).
- Features: keep serde/typeshare/ts-rs derives behind existing feature gates and in the current style; ensure feature combinations compile (no_std, std, wasm).
- Generated outputs: treat */bindings/ts/src/types.ts as generated; do not hand-edit.
- Performance: borrow over clone, avoid intermediate allocations, preallocate when sizes are known, and prefer iterators over indexing loops.
- DRY: consolidate shared logic into core/types/events-codec or dedicated helpers.
- Parity: maintain feature parity across native/wasm layers when adding SQL or Tangle APIs.
- Module layout: keep lib.rs as a module manifest and re-export surface; avoid heavy logic in lib.rs.
- Testing: add or update unit tests for new behavior and edge cases, especially around parsing, invariants, conversions, and rounding.
