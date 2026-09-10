import { defineConfig } from '@playwright/test';

const isCi = Boolean(process.env.CI);

export default defineConfig({
  testDir: '.',
  testMatch: '*.e2e.ts',
  forbidOnly: isCi,
  reporter: isCi ? 'github' : 'list',
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'bun run dev',
    cwd: '..',
    url: 'http://localhost:1420',
    reuseExistingServer: !isCi,
  },
});
