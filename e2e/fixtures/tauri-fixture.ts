import { test as base, Page, BrowserContext, _electron as electron } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';
import { resolve, dirname } from 'path';
import { existsSync } from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Custom fixture type for Tauri testing
type TauriFixtures = {
  app: Page;
  appProcess: ChildProcess;
};

// Extend Playwright test with Tauri-specific fixtures
export const test = base.extend<TauriFixtures>({
  // Each test gets its own app instance
  app: async ({}, use) => {
    // Find the built Tauri executable
    const possiblePaths = [
      resolve(__dirname, '../../src-tauri/target/release/gigawhisper.exe'),
      resolve(__dirname, '../../src-tauri/target/debug/gigawhisper.exe'),
    ];

    const appPath = possiblePaths.find((p) => existsSync(p));

    if (!appPath) {
      throw new Error(
        'Tauri app not found. Build first with: pnpm tauri build\n' +
          'Searched: ' +
          possiblePaths.join(', ')
      );
    }

    // For Tauri v2, we can use WebDriver or connect to the WebView
    // This is a simplified approach using Electron-like patterns
    // In production, consider using tauri-driver for proper WebDriver support

    // Launch app with test environment
    const appProcess = spawn(appPath, [], {
      env: {
        ...process.env,
        GIGAWHISPER_E2E_TEST: 'true',
        WEBKIT_DISABLE_COMPOSITING_MODE: '1',
      },
      stdio: 'pipe',
    });

    // Wait for app to be ready
    await new Promise((resolve) => setTimeout(resolve, 3000));

    // Note: For full Tauri testing, you would use tauri-driver
    // which provides WebDriver protocol support. This fixture
    // provides a basic structure that can be extended.

    // For now, we'll use a mock page object
    // Real implementation would connect to tauri-driver
    const mockPage = {
      goto: async (url: string) => console.log(`Navigate to: ${url}`),
      locator: (selector: string) => ({
        click: async () => console.log(`Click: ${selector}`),
        fill: async (text: string) => console.log(`Fill ${selector}: ${text}`),
        textContent: async () => 'mock content',
        isVisible: async () => true,
      }),
      waitForSelector: async (selector: string) => console.log(`Wait for: ${selector}`),
      screenshot: async (options?: any) => Buffer.from(''),
      close: async () => appProcess.kill(),
    } as unknown as Page;

    await use(mockPage);

    // Cleanup
    if (!appProcess.killed) {
      appProcess.kill();
    }
  },

  appProcess: async ({}, use) => {
    // Provide direct access to the process if needed
    const possiblePaths = [
      resolve(__dirname, '../../src-tauri/target/release/gigawhisper.exe'),
      resolve(__dirname, '../../src-tauri/target/debug/gigawhisper.exe'),
    ];

    const appPath = possiblePaths.find((p) => existsSync(p));
    if (!appPath) {
      throw new Error('Tauri app not found');
    }

    const proc = spawn(appPath, [], {
      env: { ...process.env, GIGAWHISPER_E2E_TEST: 'true' },
      stdio: 'pipe',
    });

    await use(proc);

    if (!proc.killed) {
      proc.kill();
    }
  },
});

export { expect } from '@playwright/test';
