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
- `hypercolator/` - Anchor/Solana program workspace (Task #10 complete)
- `hypercolator/programs/hypercolator/` - main on-chain program (stub, Tasks #11-13)
- `hypercolator/crates/percolator/` - vendored Percolator risk engine (pinned 719c408)
- `scripts/github/` - GitHub App automation (fork, issue, PR)
- `docs/percolator-architecture.md` - architecture reference doc
- `PLAN.md` - master project plan

## GitHub Org: hypercolator

- `github.com/hypercolator/hypercolator` - Anchor program (Rust) - LIVE
- `github.com/hypercolator/app` - Next.js frontend - placeholder
- `github.com/hypercolator/sdk` - TypeScript SDK - placeholder
- `github.com/hypercolator/bot` - Keeper bot - placeholder

GitHub App: `hypercolator-bot` (id: 3390456)
- Org installation ID: `124267092` (env: GITHUB_APP_ORG_INSTALLATION_ID)
- User installation ID: `124264528` (env: GITHUB_APP_USER_INSTALLATION_ID)

## Solana Toolchain (Task #10 complete)

- Rust: 1.88.0
- Solana CLI: 1.18.26 (at `~/.local/share/solana/install/active_release/bin/`)
- Anchor CLI: 0.30.1 (at `~/.local/bin/anchor`, symlinked from npm cache)
- `cargo test --lib`: 49/49 pass (percolator crate)
- `cargo fmt --check`: pass

PATH required for Solana/Anchor commands:
`export PATH="$HOME/.local/bin:/home/runner/.local/share/solana/install/active_release/bin:$PATH"`

## Permanent Rules (NEVER break these)

1. **No em dash or double dash in prose/text.** Never write `--` or the em dash character as punctuation in any project file, doc, comment, or string. Use single dash `-` only. Exception: CLI flags like `cargo --lib`, `pnpm --filter` are fine as they are command syntax.

2. **All project output must be in English.** Code, comments, docs, commit messages, PR descriptions, UI text - all English. This applies regardless of what language the user prompts in.

3. **Never add back button or home button to any UI.** Navigation is handled by the app shell or browser. Do not add explicit "Back" or "Home" buttons to any page or component.

4. **404 pages must be user-friendly, not developer-facing.** No stack traces, no error codes, no technical jargon. Show a clear human message and a helpful action (e.g. search, or a link to the main page). Style it consistently with the rest of the app.

5. **Never use the word "gitslop" anywhere.** Not in commit messages, code comments, UI text, docs, PR descriptions, issue bodies, or any other project file. It is permanently banned from all output.
