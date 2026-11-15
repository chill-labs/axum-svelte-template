import { defineConfig, devices } from '@playwright/test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';

const headlessShellPath = path.join(
  os.homedir(),
  'Library/Caches/ms-playwright/chromium_headless_shell-1208/chrome-headless-shell-mac-arm64/chrome-headless-shell'
);

const chromiumUse = fs.existsSync(headlessShellPath)
  ? {
      ...devices['Desktop Chrome']
    }
  : {
      ...devices['Desktop Chrome'],
      channel: 'chrome' as const
    };

export default defineConfig({
  testDir: './specs',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  timeout: 60_000,
  expect: {
    timeout: 10_000
  },
  reporter: [['list']],
  use: {
    baseURL: process.env.BASE_URL || 'http://127.0.0.1:3100',
    trace: 'retain-on-failure',
    actionTimeout: 12_000
  },
  projects: [
    {
      name: 'chromium',
      use: chromiumUse
    }
  ]
});
