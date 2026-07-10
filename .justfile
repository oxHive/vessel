_default:
  @just --choose

# --- test ---

# Rust unit + integration suites (includes binary-level e2e_loop)
[group('test')]
test:
  cargo test

# Dashboard unit tests (vitest)
[group('test')]
[working-directory: 'dashboard']
test-dashboard:
  bun run test

# Binary-level e2e only: HTTP review loop + MCP stdio
[group('test')]
e2e:
  cargo test --test e2e_loop

# Browser e2e: rebuild dashboard, drive real `vessel up` binary with Playwright
[group('test')]
[working-directory: 'dashboard']
e2e-ui:
  bun run build
  bunx playwright test

# Everything: Rust suites, dashboard unit, browser e2e
[group('test')]
test-all: test test-dashboard e2e-ui

[group('test')]
coverage:
  cargo tarpaulin --skip-clean --out Stdout

# --- lint ---

[group('lint')]
clippy:
  cargo clippy --all-targets

# --- build ---

[group('build')]
[working-directory: 'dashboard']
dashboard:
  bun run build

[group('build')]
build:
  cargo build

[group('build')]
install:
  cargo install --path . --force

# --- release ---

[group('release')]
release-major:
  just _release major

[group('release')]
release-minor:
  just _release minor

[group('release')]
release-patch:
  just _release patch

# Release a new version: just release patch|minor|major
_release bump:
  cargo release --execute {{bump}}
