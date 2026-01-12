import { Mic, Loader2, AlertCircle } from 'lucide-react';

interface RecordingState {
  state: 'idle' | 'recording' | 'processing' | 'error';
  duration_ms?: number;
  error?: string;
}

interface RecordingIndicatorProps {
  state: RecordingState;
}

export function RecordingIndicator({ state }: RecordingIndicatorProps) {
  const formatDuration = (ms: number) => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="flex items-center gap-4">
      {/* Icon */}
      <div
        className={`w-12 h-12 rounded-full flex items-center justify-center ${
          state.state === 'idle'
            ? 'bg-gray-100 dark:bg-gray-700 text-gray-500'
            : state.state === 'recording'
            ? 'bg-red-100 dark:bg-red-900/30 text-red-600 animate-pulse'
            : state.state === 'processing'
            ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600'
            : 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-600'
        }`}
      >
        {state.state === 'idle' && <Mic className="w-6 h-6" />}
        {state.state === 'recording' && <Mic className="w-6 h-6" />}
        {state.state === 'processing' && <Loader2 className="w-6 h-6 animate-spin" />}
        {state.state === 'error' && <AlertCircle className="w-6 h-6" />}
      </div>

      {/* Status Text */}
      <div>
        <div className="font-medium capitalize">{state.state}</div>
        {state.state === 'recording' && state.duration_ms !== undefined && (
          <div className="text-sm text-gray-500 dark:text-gray-400">
            {formatDuration(state.duration_ms)}
          </div>
        )}
        {state.state === 'error' && state.error && (
          <div className="text-sm text-red-600 dark:text-red-400">
            {state.error}
          </div>
        )}
      </div>
    </div>
  );
}
