import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Download, X, RefreshCw, CheckCircle } from 'lucide-react';

interface UpdateInfo {
  currentVersion: string;
  newVersion: string;
  body?: string;
  variant?: string;
}

interface DownloadProgress {
  downloaded: number;
  total?: number;
  percent?: number;
}

type UpdateState = 'available' | 'downloading' | 'ready' | 'hidden';

export function UpdateNotification() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [state, setState] = useState<UpdateState>('hidden');
  const [progress, setProgress] = useState<number>(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Listen for update available event
    const unsubscribeAvailable = listen<UpdateInfo>('update-available', (event) => {
      setUpdateInfo(event.payload);
      setState('available');
      setError(null);
    });

    // Listen for download progress
    const unsubscribeProgress = listen<DownloadProgress>('update-download-progress', (event) => {
      if (event.payload.percent) {
        setProgress(event.payload.percent);
      }
    });

    // Listen for update installed - auto-restart if auto_update is enabled
    const unsubscribeInstalled = listen('update-installed', async () => {
      setState('ready');
      setProgress(100);
    });

    // Listen for auto-restart: when update is ready and auto_update is on
    const unsubscribeAutoRestart = listen('update-installed', async () => {
      // Small delay to let the UI update before restarting
      await new Promise((r) => setTimeout(r, 2000));
      // Re-check setting at event time via fresh invoke
      try {
        const currentSettings = await invoke<{ ui: { auto_update: boolean } }>('get_settings');
        if (currentSettings.ui.auto_update) {
          await invoke('restart_app');
        }
      } catch {
        // Ignore - user will see the restart button
      }
    });

    return () => {
      unsubscribeAvailable.then((fn) => fn());
      unsubscribeProgress.then((fn) => fn());
      unsubscribeInstalled.then((fn) => fn());
      unsubscribeAutoRestart.then((fn) => fn());
    };
  }, []);

  const handleInstall = async () => {
    setState('downloading');
    setProgress(0);
    setError(null);

    try {
      await invoke('install_update');
    } catch (e) {
      setError(e as string);
      setState('available');
    }
  };

  const handleRestart = async () => {
    await invoke('restart_app');
  };

  const handleDismiss = () => {
    setState('hidden');
  };

  if (state === 'hidden' || !updateInfo) {
    return null;
  }

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-sm animate-slide-up">
      <div className="card overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 bg-accent-subtle border-b border-edge">
          <div className="flex items-center gap-2">
            {state === 'ready' ? (
              <CheckCircle className="w-5 h-5 text-green-500" />
            ) : (
              <Download className="w-5 h-5 text-accent" />
            )}
            <span className="font-semibold text-sm">
              {state === 'ready' ? 'Update Ready' : 'Update Available'}
            </span>
          </div>
          {state !== 'downloading' && (
            <button
              onClick={handleDismiss}
              className="p-1.5 hover:bg-surface-tertiary rounded-lg transition-colors duration-150"
            >
              <X className="w-4 h-4 text-content-tertiary" />
            </button>
          )}
        </div>

        {/* Content */}
        <div className="px-4 py-3">
          <p className="text-sm text-content-secondary mb-3 leading-relaxed">
            {state === 'ready' ? (
              'The update has been downloaded. Restart to apply.'
            ) : (
              <>
                Version <span className="font-mono font-medium text-content-primary">{updateInfo.newVersion}</span> is available.
                <br />
                <span className="text-content-tertiary">Current: {updateInfo.currentVersion}</span>
              </>
            )}
          </p>

          {/* Progress bar */}
          {state === 'downloading' && (
            <div className="mb-3">
              <div className="progress-bar">
                <div
                  className="progress-bar-fill"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="text-xs text-content-secondary mt-1.5">Downloading... {progress}%</p>
            </div>
          )}

          {/* Error message */}
          {error && (
            <p className="text-sm text-red-500 mb-3">{error}</p>
          )}

          {/* Actions */}
          <div className="flex gap-2">
            {state === 'available' && (
              <>
                <button
                  onClick={handleInstall}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-gradient-to-r from-indigo-500 to-violet-500 text-white rounded-lg hover:from-indigo-600 hover:to-violet-600 text-sm font-medium transition-colors duration-150"
                >
                  <Download className="w-4 h-4" />
                  Update Now
                </button>
                <button
                  onClick={handleDismiss}
                  className="px-3 py-2 text-content-secondary hover:bg-surface-tertiary rounded-lg text-sm transition-colors duration-150"
                >
                  Later
                </button>
              </>
            )}

            {state === 'downloading' && (
              <div className="flex-1 flex items-center justify-center py-2 text-content-secondary text-sm">
                <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                Downloading...
              </div>
            )}

            {state === 'ready' && (
              <button
                onClick={handleRestart}
                className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 text-sm font-medium transition-colors duration-150"
              >
                <RefreshCw className="w-4 h-4" />
                Restart Now
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
