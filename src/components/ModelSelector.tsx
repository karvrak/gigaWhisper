import { Download, Check, Loader2, Trash2, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type WhisperModel = 'tiny' | 'base' | 'small' | 'medium' | 'large';

interface ModelSelectorProps {
  value: WhisperModel;
  onChange: (value: WhisperModel) => void;
}

interface ModelInfo {
  model: WhisperModel;
  path: string;
  size_bytes: number;
  downloaded: boolean;
}

interface DownloadProgress {
  model: string;
  downloaded_bytes: number;
  total_bytes: number;
  percentage: number;
  speed_bps: number;
}

const modelDescriptions: Record<WhisperModel, string> = {
  tiny: 'Fastest, lower accuracy',
  base: 'Good balance of speed and accuracy',
  small: 'Better accuracy, moderate speed',
  medium: 'High accuracy, slower',
  large: 'Best accuracy, requires GPU',
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatSpeed(bps: number): string {
  if (bps < 1024) return `${bps} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
}

export function ModelSelector({ value, onChange }: ModelSelectorProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Load models on mount
  useEffect(() => {
    loadModels();
  }, []);

  // Listen for download events
  useEffect(() => {
    const unlistenProgress = listen<DownloadProgress>('model-download-progress', (event) => {
      setProgress(event.payload);
    });

    const unlistenComplete = listen<{ model: string; path: string }>('model-download-complete', () => {
      setDownloading(null);
      setProgress(null);
      loadModels(); // Refresh model list
    });

    const unlistenError = listen<{ model: string; error: string }>('model-download-error', (event) => {
      setDownloading(null);
      setProgress(null);
      setError(`Download failed: ${event.payload.error}`);
    });

    const unlistenCancelled = listen<{ model: string }>('model-download-cancelled', () => {
      setDownloading(null);
      setProgress(null);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
      unlistenCancelled.then((fn) => fn());
    };
  }, []);

  const loadModels = async () => {
    try {
      const modelList = await invoke<ModelInfo[]>('list_models');
      setModels(modelList);
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  };

  const handleDownload = async (model: WhisperModel) => {
    setError(null);
    setDownloading(model);
    try {
      await invoke('download_model', { model });
    } catch (e) {
      setError(`Download failed: ${e}`);
      setDownloading(null);
    }
  };

  const handleDelete = async (model: WhisperModel) => {
    try {
      await invoke('delete_model', { model });
      loadModels();
    } catch (e) {
      setError(`Delete failed: ${e}`);
    }
  };

  const handleCancelDownload = async (model: WhisperModel) => {
    try {
      await invoke('cancel_model_download', { model });
    } catch (e) {
      console.error('Failed to cancel download:', e);
    }
  };

  return (
    <div className="space-y-2" role="radiogroup" aria-label="Select Whisper model">
      {error && (
        <div className="p-2 text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 rounded-lg" role="alert">
          {error}
        </div>
      )}
      {models.map((model) => {
        const isDownloading = downloading === model.model;
        const currentProgress = isDownloading && progress?.model === model.model ? progress : null;

        return (
          <div
            key={model.model}
            role="radio"
            aria-checked={value === model.model}
            aria-disabled={!model.downloaded && !isDownloading}
            tabIndex={model.downloaded ? 0 : -1}
            className={`flex items-center justify-between p-3 border rounded-lg cursor-pointer transition-colors ${
              value === model.model
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
            } ${!model.downloaded && !isDownloading ? 'opacity-75' : ''}`}
            onClick={() => model.downloaded && onChange(model.model)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                if (model.downloaded) onChange(model.model);
              }
            }}
          >
            <div className="flex items-center gap-3 flex-1 min-w-0">
              {/* Selection indicator */}
              <div
                className={`w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
                  value === model.model
                    ? 'border-blue-500 bg-blue-500'
                    : 'border-gray-300 dark:border-gray-600'
                }`}
              >
                {value === model.model && <Check className="w-3 h-3 text-white" />}
              </div>

              {/* Model info */}
              <div className="min-w-0 flex-1">
                <div className="font-medium capitalize">
                  {model.model}{' '}
                  <span className="text-sm text-gray-500 dark:text-gray-400">
                    ({formatBytes(model.size_bytes)})
                  </span>
                </div>
                <div className="text-sm text-gray-500 dark:text-gray-400 truncate">
                  {modelDescriptions[model.model]}
                </div>
                {/* Download progress */}
                {isDownloading && currentProgress && (
                  <div className="mt-2">
                    <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mb-1">
                      <span>{currentProgress.percentage.toFixed(1)}%</span>
                      <span>{formatSpeed(currentProgress.speed_bps)}</span>
                    </div>
                    <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                      <div
                        className="bg-blue-500 h-1.5 rounded-full transition-all"
                        style={{ width: `${currentProgress.percentage}%` }}
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Download button, status, or delete */}
            <div className="flex items-center gap-2 ml-2 flex-shrink-0">
              {isDownloading ? (
                <div className="flex items-center gap-2">
                  <span className="flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400">
                    <Loader2 className="w-4 h-4 animate-spin" />
                  </span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCancelDownload(model.model);
                    }}
                    className="p-1 text-gray-400 hover:text-red-500 transition-colors"
                    title="Cancel download"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
              ) : model.downloaded ? (
                <>
                  <span className="text-sm text-green-600 dark:text-green-400 flex items-center gap-1">
                    <Check className="w-4 h-4" />
                    Ready
                  </span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(model.model);
                    }}
                    className="p-1 text-gray-400 hover:text-red-500 transition-colors"
                    title="Delete model"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </>
              ) : (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDownload(model.model);
                  }}
                  className="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400"
                >
                  <Download className="w-4 h-4" />
                  Download
                </button>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
