# M1 · Step 1 — App scaffold (Tauri v2 + React + TS) and CI

## Goal
An empty Tauri v2 app in `app/` that builds, runs, and tests, with CI.

## Spec refs
§3 (architecture), §3.1, D1, D4.

## Depends on
— (independent of M0).

## Tasks
- Check the current scaffolding command with Context7 (Tauri v2: `pnpm create tauri-app` with the React + TypeScript + Vite template,
  pnpm as the package manager). Create it in `app/` (non-interactive flags; if the CLI insists on prompts, scaffold into the scratchpad and
  move the files). App identifier `dev.dericbourg.madsister`, product name `madsister`.
- `tsconfig.json`: `strict: true` (template default; check it). Remove the template demo (greet command, logos, CSS).
- vitest + jsdom + `@testing-library/react` + `@testing-library/user-event`; script `pnpm test` = `vitest run`.
- Scripts: `pnpm build` (tsc + vite build), `pnpm tauri dev`.
- Folder layout inside `app/src/`: `model/` (pure TS: song, chords, commands — no React), `ui/` (components), `App.tsx`.
- **CI** `.github/workflows/app.yml`: on push/PR touching `app/**` or the workflow; `ubuntu-latest`; `pnpm/action-setup` +
  `actions/setup-node` (pnpm cache); `pnpm install --frozen-lockfile`, `pnpm test`, `pnpm build`; then Rust:
  install the Tauri Linux system deps (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf` — take the
  current list from the Tauri v2 prerequisites docs), `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache` on `app/src-tauri`,
  `cargo check --locked` + `cargo clippy -- -D warnings` in `app/src-tauri`. No bundling in CI (that's M6).
- Add `app/src-tauri/target/`, `app/node_modules/`, `app/dist/` to `.gitignore` if not already there.

## Tests
- One trivial render test of `App` (it renders a heading "madsister"), proving the test harness works.

## Verify
```
cd app && pnpm install && pnpm test && pnpm build
cd src-tauri && cargo check && cargo clippy -- -D warnings
uvx --from actionlint-py actionlint ../../.github/workflows/app.yml
```
Optionally `pnpm tauri dev` (just check it starts without error, then kill it; it opens a window).

## Commit message
`feat(app): scaffold Tauri v2 React app with CI`

## Out of scope
Any feature.
