# Rad Roots Application - Agent Specification

## 1. Scope and hierarchy

- This file applies to the full repository.
- Keep this file concise, durable, and repository-specific.
- If a closer directory-level `AGENTS.md` is added later, it overrides this file for that subtree.
- Put detailed procedures, examples, and temporary migration notes in other checked-in docs, not here.

## 2. Repository identity

- This repository is the standalone Rad Roots application repository.
- Optimize for durable application structure, explicit boundaries, portability, and clear runtime behavior.
- Treat this as a public open-source application project: commits, docs, and handoff language should read clearly to contributors who only know this repository.
- Preserve the repository’s top-level identity files and keep the workspace easy to understand from the root.

## 3. Change discipline

- Prefer the smallest coherent change that fully addresses the request.
- Do not mix unrelated cleanup, speculative refactors, or roadmap work into the same change.
- Prefer clean target-state changes over temporary compatibility layers unless compatibility is explicitly required.
- Remove obsolete code, dependencies, and scaffolding when they are clearly replaced.
- Do not leave hidden task trackers in source comments, markdown checklists, or stray notes.

## 4. Before editing

Before making substantial changes:

- Read this file, `README.md`, and `CONTRIBUTING.md`.
- Inspect `git status --short` before broad edits, refactors, or file removals.
- Read the current implementation and nearby tests before changing behavior.
- Use checked-in documentation and commands as the source of truth.
- Do not assume contributor-specific local tooling or machine setup beyond what the repository documents.
- Surface blockers early when the task depends on unresolved product decisions, missing prerequisites, or unclear architecture boundaries.

## 5. Validation and command surface

- Run validation from the repository root.
- Prefer the narrowest relevant validation for the files or crate being changed.
- Use documented commands first. When no narrower repo-specific command is documented, use standard Cargo commands such as:
  - `cargo metadata --format-version 1 --no-deps`
  - `cargo check`
  - targeted `cargo test`
  - targeted `cargo run -p radroots_studio_app_desktop`
- If validation cannot be run, report the blocker clearly.

## 6. Workspace structure

- Keep the repository root as the workspace root.
- Keep reusable Rust application logic under `crates/shared/`.
- Keep Rust host-integration adapters under `crates/bridges/`.
- Keep runnable Rust targets under `crates/launchers/`.
- Keep reusable platform-native bridge libraries under `native/bridges/`.
- Keep native host projects under `platforms/`.
- Add new crates only when they represent a durable architectural boundary.
- Keep manifests, paths, and crate boundaries simple and intentional.
- Do not reintroduce obsolete framework scaffolding unless the requested change explicitly requires it.

## 7. Rust engineering rules

- Use Rust `1.92.0`, edition `2024`, and workspace dependency versions from the root `Cargo.toml`.
- Prefer safe, explicit APIs and avoid `unsafe`.
- Keep state, data flow, and side effects understandable; prefer typed models and explicit transitions over stringly APIs or loosely typed maps.
- Avoid hidden panics in non-test code.
- Keep module layout and manifests clean; remove dead dependencies, dead modules, and unused feature wiring when they are no longer needed.
- Keep code readable and direct; avoid unnecessary abstraction in early-stage application code.
- Add or update deterministic tests when behavior, invariants, parsing, or state transitions change.

## 8. Dependency rules

- Prefer root workspace dependencies where possible.
- Use canonical upstream crate names in manifests and code.
- Prefer dependency choices that align with the existing Rad Roots Rust ecosystem when practical.
- Introduce new dependencies only when they are justified by a clear product or architectural need.

## 9. Commit and handoff rules

- Format commits as `<scope>: <imperative summary>`.
- Use lowercase scopes.
- Split unrelated changes into separate commits.
- Keep commit messages and handoff summaries clear and standalone.
- In handoff, state what changed, what validation ran, and any remaining risks or assumptions.

## 10. Definition of done

- The requested change is implemented.
- Replaced or obsolete scaffolding is removed when no longer needed.
- Relevant validation ran, or a concrete blocker is reported.
- Any affected documentation or structural context is updated with the code change when necessary.
