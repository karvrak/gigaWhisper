import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e/tests',
  fullyParallel: false, // Tauri tests should run sequentially
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Single worker for Tauri app
  reporter: [
    ['html', { open: 'never' }],
    ['list'],
  ],
  timeout: 60000, // 60s timeout for app startup
  expect: {
    timeout: 10000,
  },
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  globalSetup: './e2e/setup/global-setup.ts',
  globalTeardown: './e2e/setup/global-teardown.ts',
  projects: [
    {
      name: 'gigawhisper',
      use: {
        // Tauri WebView testing config
        baseURL: 'tauri://localhost',
      },
    },
  ],
});
