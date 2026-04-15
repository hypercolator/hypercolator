# Workspace

## Overview

pnpm workspace monorepo using TypeScript. Each package manages its own dependencies.

## Stack

- **Monorepo tool**: pnpm workspaces
- **Node.js version**: 24
- **Package manager**: pnpm
- **TypeScript version**: 5.9
- **API framework**: Express 5
- **Database**: PostgreSQL + Drizzle ORM
- **Validation**: Zod (`zod/v4`), `drizzle-zod`
- **API codegen**: Orval (from OpenAPI spec)
- **Build**: esbuild (CJS bundle)

## Key Commands

- `pnpm run typecheck` - full typecheck across all packages
- `pnpm run build` - typecheck + build all packages
- `pnpm --filter @workspace/api-spec run codegen` - regenerate API hooks and Zod schemas from OpenAPI spec
- `pnpm --filter @workspace/db run push` - push DB schema changes (dev only)
- `pnpm --filter @workspace/api-server run dev` - run API server locally

See the `pnpm-workspace` skill for workspace structure, TypeScript setup, and package details.

## Project: Hypercolator

Decentralized perpetual futures DEX on Solana. Permissionless market creation for any token including pump.fun tokens. Self-funded insurance model via trading fees. Based on the Percolator risk engine by Anatoly Yakovenko.

Key directories:
- `hypercolator/` - Anchor/Solana program workspace
- `hypercolator/programs/hypercolator/` - main on-chain program
- `hypercolator/crates/percolator/` - vendored Percolator risk engine library
- `scripts/github/` - GitHub App automation (fork, issue, PR)
- `scripts/keeper/` - keeper bot (TWAP crank, liquidations, expiry)
- `docs/percolator-architecture.md` - architecture reference doc
- `PLAN.md` - master project plan

## Permanent Rules (NEVER break these)

1. **No em dash or double dash in prose/text.** Never write `--` or the em dash character as punctuation in any project file, doc, comment, or string. Use single dash `-` only. Exception: CLI flags like `cargo --lib`, `pnpm --filter` are fine as they are command syntax.

2. **All project output must be in English.** Code, comments, docs, commit messages, PR descriptions, UI text - all English. This applies regardless of what language the user prompts in.

3. **Never add back button or home button to any UI.** Navigation is handled by the app shell or browser. Do not add explicit "Back" or "Home" buttons to any page or component.

4. **404 pages must be user-friendly, not developer-facing.** No stack traces, no error codes, no technical jargon. Show a clear human message and a helpful action (e.g. search, or a link to the main page). Style it consistently with the rest of the app.
