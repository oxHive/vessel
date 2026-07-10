_default:
  @just --choose

test:
  cargo test

[working-directory: 'dashboard']
dashboard:
  bun run build

# Browser e2e: build the dashboard (debug binaries serve dist from disk),
# then drive the real `vessel up` binary with Playwright
[working-directory: 'dashboard']
e2e-ui:
  npm run build
  npx playwright test

build:
  cargo build

install:
  cargo install --path . --force

release-major:
  just _release major

release-minor:
  just _release minor

release-patch:
  just _release patch

# Release a new version: just release patch|minor|major
_release bump:
  cargo release --execute {{bump}}

clippy:
  cargo clippy --all-targets

coverage:
  cargo tarpaulin --skip-clean --out Stdout
