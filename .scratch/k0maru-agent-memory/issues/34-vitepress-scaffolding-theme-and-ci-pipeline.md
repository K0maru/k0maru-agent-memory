# Ticket 34: VitePress Scaffolding, Dark Cockpit Theme & CI/CD Pipeline

**Status**: closed
**Blocked by**: None

## Goal
Scaffold the modern VitePress documentation engine, configure bilingual internationalization (`/zh/` and `/en/`), implement the Developer Dark Cockpit aesthetic theme matching `k0maru ui`, and configure the GitHub Actions workflow for automated deployment to GitHub Pages.

## Tasks
1. Initialize root `package.json` with VitePress (`vitepress`, `@types/node`).
2. Add npm scripts:
   - `"docs:dev": "vitepress dev docs"`
   - `"docs:build": "vitepress build docs"`
   - `"docs:preview": "vitepress preview docs"`
3. Create `docs/.vitepress/config.mts`:
   - Dual-locale routing: `zh` (root `/zh/`, label `简体中文`) and `en` (root `/en/`, label `English`).
   - Symmetrical navbars and sidebars for both locales.
   - Built-in Minisearch local search configuration (`provider: 'local'`).
   - Social links to GitHub repository (`https://github.com/K0maru/k0maru-agent-memory`).
4. Create `docs/.vitepress/theme/index.ts` and `docs/.vitepress/theme/custom.css`:
   - Developer Dark Cockpit styling: `#0F172A` background, `#1B2336` card surfaces, `#22C55E` emerald accent, `#334155` borders, and `JetBrains Mono` code blocks.
5. Create `.github/workflows/deploy-docs.yml`:
   - Builds VitePress docs on push to `main`.
   - Deploys static output to GitHub Pages via `actions/deploy-pages`.
6. Verify local build via `npm run docs:build`.
