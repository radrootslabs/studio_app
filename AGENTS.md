# Rad Roots - Agricultural Network Application

#### Code Rules

1. TypeScript
- Use strict TypeScript at all times.
- Never use as any, any, unknown as, or any weakening of types.
- Prefer explicit interfaces and types.
- Use enums or literal unions when appropriate.
- Avoid default exports unless legacy structure requires it.

2. Source Code Output
- No inline comments, block comments, or JSDoc.
- Do not generate explanatory comments in any code.
- Code must stay deterministic and reproducible.
- Write source code using snake_case conventions, while type names use PascalCase.

1. DRY and Modularity
- Do not duplicate logic within or across projects.
- Extract shared logic into the appropriate package under packages/.
- Prefer small, focused modules.
- Shared utilities must live in shared packages, not in apps.

1. Functional and Architectural Cleanliness
- Prefer pure functions.
- Prefer composition over inheritance.
- Avoid side effects unless explicitly needed.
- Avoid global mutable state.

 
