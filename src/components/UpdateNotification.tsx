import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Download, X, RefreshCw, CheckCircle } from 'lucide-react';

interface UpdateInfo {
  currentVersion: string;
  newVersion: string;
  body?: string;
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

    // Listen for update installed
    const unsubscribeInstalled = listen('update-installed', () => {
      setState('ready');
      setProgress(100);
    });

    return () => {
      unsubscribeAvailable.then((fn) => fn());
      unsubscribeProgress.then((fn) => fn());
      unsubscribeInstalled.then((fn) => fn());
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
    <div className="fixed bottom-4 right-4 z-50 max-w-sm">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 bg-blue-50 dark:bg-blue-900/30 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            {state === 'ready' ? (
              <CheckCircle className="w-5 h-5 text-green-600" />
            ) : (
              <Download className="w-5 h-5 text-blue-600" />
            )}
            <span className="font-medium text-sm">
              {state === 'ready' ? 'Update Ready' : 'Update Available'}
            </span>
          </div>
          {state !== 'downloading' && (
            <button
              onClick={handleDismiss}
              className="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* Content */}
        <div className="px-4 py-3">
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
            {state === 'ready' ? (
              'The update has been downloaded. Restart to apply.'
            ) : (
              <>
                Version <span className="font-mono font-medium">{updateInfo.newVersion}</span> is available.
                <br />
                <span className="text-gray-500">Current: {updateInfo.currentVersion}</span>
              </>
            )}
          </p>

          {/* Progress bar */}
          {state === 'downloading' && (
            <div className="mb-3">
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="text-xs text-gray-500 mt-1">Downloading... {progress}%</p>
            </div>
          )}

          {/* Error message */}
          {error && (
            <p className="text-sm text-red-600 dark:text-red-400 mb-2">{error}</p>
          )}

          {/* Actions */}
          <div className="flex gap-2">
            {state === 'available' && (
              <>
                <button
                  onClick={handleInstall}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 text-sm font-medium"
                >
                  <Download className="w-4 h-4" />
                  Update Now
                </button>
                <button
                  onClick={handleDismiss}
                  className="px-3 py-2 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md text-sm"
                >
                  Later
                </button>
              </>
            )}

            {state === 'downloading' && (
              <div className="flex-1 flex items-center justify-center py-2 text-gray-500 text-sm">
                <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                Downloading...
              </div>
            )}

            {state === 'ready' && (
              <button
                onClick={handleRestart}
                className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 text-sm font-medium"
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
