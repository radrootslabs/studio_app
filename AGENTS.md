# Rad Roots - Code Directives

1. TypeScript
- Use strict TypeScript.
- Do not use any, unknown casts, or weaken types.
- Prefer explicit interfaces and types.
- Use enums or literal unions when clear.
- Use named exports; avoid default exports.

2. Naming
- Variables and functions use snake_case.
- Types, interfaces, and enums use PascalCase.
- Prefer layered prefixes to namespace meaning: domain_object_action, e.g. nostr_keys_gen(), NostrKeysGen.
- Do not enforce naming conventions in _env*.ts files.
- Constant data structures must use ALL_CAPS_SNAKE_CASE.

3. Source Code
- Keep code deterministic and reproducible.
- Do not add source code comments.
- Single-line if statements must not use braces.
- Anchor comments are allowed only when they start with @ (e.g. // @todo).
- /* */ blocks are allowed only to disable features during development and must not include descriptive text.
- <!-- --> HTML blocks are allowed only to disable features during development or compiler/lint suppression (e.g. <!-- svelte-ignore ... -->).

1. Modularity
- Do not duplicate logic.
- Put shared or generalizable code in packages/.
- Apps should rely on packages/ for shared utilities.
- Treat @radroots/*-bindings as generated from .rs crates; do not edit or format their .ts outputs. If issues arise, change upstream .rs or report the error instead.
- Every class must implement a same-name interface prefixed with I; all public methods must be declared on the interface, and method return types are required for all class methods.

1. Architecture
- Prefer pure functions.
- Prefer composition over inheritance.
- Avoid side effects unless required.
- Avoid global mutable state.

1. Change Policy
- Apply breaking changes when needed.
- Do not add legacy or shim fixes.
