async function globalTeardown() {
  const tauriProcess = (global as any).__TAURI_PROCESS__;

  if (tauriProcess) {
    console.log('Stopping Tauri app...');

    // Kill the process gracefully
    tauriProcess.kill('SIGTERM');

    // Wait a bit for graceful shutdown
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        // Force kill if still running
        if (!tauriProcess.killed) {
          tauriProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);

      tauriProcess.on('exit', () => {
        clearTimeout(timeout);
        resolve();
      });
    });

    console.log('Tauri app stopped');
  }
}

export default globalTeardown;
