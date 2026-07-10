import { defineConfig } from '@playwright/test'
import os from 'node:os'
import path from 'node:path'
import { E2E_HOME, PORT } from './e2e/helpers'

// The server child gets an isolated HOME so no user data is touched, but
// cargo still needs its real registry/toolchain locations.
const realHome = os.homedir()

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.e2e.ts',
  workers: 1,
  fullyParallel: false,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
  },
  webServer: {
    // Debug builds serve dashboard assets from disk (rust-embed default), so
    // run `npm run build` first — `just e2e-ui` does both.
    command: `cargo run --quiet -- up --port ${PORT}`,
    cwd: path.join(__dirname, '..'),
    url: `http://127.0.0.1:${PORT}/health`,
    reuseExistingServer: false,
    timeout: 180_000,
    env: {
      ...process.env,
      HOME: E2E_HOME,
      XDG_DATA_HOME: path.join(E2E_HOME, '.local/share'),
      XDG_CONFIG_HOME: path.join(E2E_HOME, '.config'),
      CARGO_HOME: process.env.CARGO_HOME ?? path.join(realHome, '.cargo'),
      RUSTUP_HOME: process.env.RUSTUP_HOME ?? path.join(realHome, '.rustup'),
    },
  },
})
