# Contributing

Rad Roots is an open-source application. Contributions are welcome, including bug fixes, usability improvements, documentation updates, tests, and new features.

## Scope

This repository is the standalone Rad Roots application repository. Shared Rust application code is organized under `crates/`. Reusable native libraries are organized under `native/`, and native host projects are organized under `platforms/`.

## Prerequisites

Install the Rust toolchain used by this repository:

```bash
rustup toolchain install 1.92.0
rustup target add wasm32-unknown-unknown
```

Install Trunk for the wasm target:

```bash
cargo install trunk
```

On hosts that will build or run the Android shell, ensure Java 17 or newer is available. The Android scripts bootstrap the local Gradle, SDK, NDK, `cargo-ndk`, and emulator resources into `platforms/android/.tooling` on demand.

On macOS, ensure the Apple Swift toolchain is available. The desktop target links the shared Apple native security package during build.

Confirm your environment:

```bash
cargo --version
rustc --version
trunk --version
java --version
```

On macOS, also confirm:

```bash
swift --version
```

## Getting Started

Clone your fork and enter the repository root:

```bash
git clone https://github.com/<YOUR-USERNAME>/app.git
cd app
```

To use the repository-pinned toolchain:

```bash
rustup override set 1.92.0
```

## Development Commands

Run these commands from the repository root.

Inspect workspace metadata:

```bash
cargo metadata --format-version 1 --no-deps
```

Check the application:

```bash
cargo check
```

Run tests:

```bash
cargo test
```

Run the native application:

```bash
cargo run -p radroots-app-desktop
```

Check the Android target:

```bash
./scripts/check-android-target.sh
```

Build the Android host:

```bash
./scripts/build-android-host.sh
```

Run the Android app in the emulator:

```bash
./scripts/run-android-emulator.sh
```

Check the wasm application:

```bash
./scripts/with-wasm-toolchain.sh env -u NO_COLOR cargo check -p radroots-app-web --target wasm32-unknown-unknown
```

Build the wasm application:

```bash
cd crates/web
../../scripts/with-wasm-toolchain.sh env -u NO_COLOR trunk build
```

Run the wasm application:

```bash
cd crates/web
../../scripts/with-wasm-toolchain.sh env -u NO_COLOR trunk serve --open
```

Test the Apple native security package:

```bash
cd native/apple/swift/RadRootsAppleSecurity
swift test
```

## Contribution Guidelines

- Keep changes scoped to a single coherent change.
- Prefer small, reviewable commits.
- Update tests when behavior changes.
- Update documentation when commands, structure, or contributor workflow changes.
- Use repo-relative paths in docs, comments, and contributor-facing text.
- Keep documentation path references relative to this repository root.
- Do not use absolute filesystem paths or home-directory path forms in repository docs.
- Remove obsolete code and dependencies when they are clearly replaced.
- Use workspace-managed dependency versions from the root `Cargo.toml`.

## Reporting Issues

When reporting a bug, include:

- your operating system and version
- Rust toolchain version
- the command you ran
- the observed behavior
- the expected behavior
- logs, screenshots, or backtraces if available

## Submitting Changes

1. Create a branch for your change.
2. Make the smallest coherent update that solves the issue.
3. Run the relevant validation commands from this document.
4. Open a pull request with a clear summary of what changed and how it was verified.

## Code of Conduct

Be respectful, direct, and constructive in issues and reviews.

## License

By contributing to this repository, you agree that your contributions will be distributed under the repository's license. See [LICENSE](LICENSE).
