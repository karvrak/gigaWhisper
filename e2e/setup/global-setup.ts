import { spawn, ChildProcess } from 'child_process';
import { resolve, dirname } from 'path';
import { existsSync } from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

let tauriProcess: ChildProcess | null = null;

async function globalSetup() {
  // Find the built Tauri executable
  const possiblePaths = [
    resolve(__dirname, '../../src-tauri/target/release/gigawhisper.exe'),
    resolve(__dirname, '../../src-tauri/target/debug/gigawhisper.exe'),
  ];

  const appPath = possiblePaths.find((p) => existsSync(p));

  if (!appPath) {
    console.error('Tauri app not found. Please build first with: pnpm tauri build');
    console.error('Searched paths:', possiblePaths);
    throw new Error('Tauri executable not found');
  }

  console.log(`Starting Tauri app from: ${appPath}`);

  // Start the Tauri app
  tauriProcess = spawn(appPath, [], {
    env: {
      ...process.env,
      // Set test mode environment variable
      GIGAWHISPER_E2E_TEST: 'true',
    },
    stdio: 'pipe',
  });

  tauriProcess.stdout?.on('data', (data) => {
    console.log(`[Tauri] ${data}`);
  });

  tauriProcess.stderr?.on('data', (data) => {
    console.error(`[Tauri Error] ${data}`);
  });

  // Wait for app to start
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error('Tauri app failed to start within timeout'));
    }, 30000);

    // Simple delay to let the app initialize
    // In production, you'd want to poll for readiness
    setTimeout(() => {
      clearTimeout(timeout);
      resolve();
    }, 5000);
  });

  // Store process reference for teardown
  (global as any).__TAURI_PROCESS__ = tauriProcess;

  console.log('Tauri app started successfully');
}

export default globalSetup;
