import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Copy, Trash2, Clock, RefreshCw, Play, Square, AlertTriangle } from 'lucide-react';

interface HistoryEntry {
  id: string;
  text: string;
  timestamp: string;
  duration_ms: number;
  provider: string;
  language: string | null;
  audio_path: string | null;
}

export function HistoryPanel() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [loadingAudioId, setLoadingAudioId] = useState<string | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const loadHistory = async () => {
    try {
      const history = await invoke<HistoryEntry[]>('get_transcription_history');
      setEntries(history);
    } catch (e) {
      console.error('Failed to load history:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadHistory();

    // Listen for history updates
    const unsubscribe = listen('history:updated', () => {
      loadHistory();
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  const copyToClipboard = async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  };

  const deleteEntry = async (id: string) => {
    try {
      await invoke('delete_history_entry', { id });
      setEntries((prev) => prev.filter((e) => e.id !== id));
      // Stop playing if this entry was playing
      if (playingId === id) {
        stopAudio();
      }
    } catch (e) {
      console.error('Failed to delete:', e);
    }
  };

  const playAudio = async (id: string) => {
    try {
      // Stop current audio if playing
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current = null;
      }

      setLoadingAudioId(id);

      // Get audio data as base64
      const audioData = await invoke<string>('get_audio_data', { id });

      const audio = new Audio(audioData);
      audioRef.current = audio;

      audio.onended = () => {
        setPlayingId(null);
        audioRef.current = null;
      };

      audio.onerror = () => {
        console.error('Audio playback error');
        setPlayingId(null);
        audioRef.current = null;
      };

      await audio.play();
      setPlayingId(id);
    } catch (e) {
      console.error('Failed to play audio:', e);
    } finally {
      setLoadingAudioId(null);
    }
  };

  const stopAudio = () => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current = null;
    }
    setPlayingId(null);
  };

  const toggleAudio = (id: string) => {
    if (playingId === id) {
      stopAudio();
    } else {
      playAudio(id);
    }
  };

  const clearAllHistory = async () => {
    try {
      stopAudio();
      await invoke('clear_history');
      setEntries([]);
      setShowClearConfirm(false);
    } catch (e) {
      console.error('Failed to clear history:', e);
    }
  };

  const formatTimestamp = (timestamp: string) => {
    try {
      const date = new Date(timestamp);
      return date.toLocaleString();
    } catch {
      return timestamp;
    }
  };

  const formatDuration = (ms: number) => {
    const seconds = Math.round(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}m ${secs}s`;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <RefreshCw className="w-6 h-6 animate-spin text-gray-400" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-medium">Transcription History</h2>
        {entries.length > 0 && (
          <button
            onClick={() => setShowClearConfirm(true)}
            className="text-sm text-red-500 hover:text-red-600 flex items-center gap-1"
          >
            <Trash2 className="w-4 h-4" />
            Clear All
          </button>
        )}
      </div>

      {/* Empty state */}
      {entries.length === 0 && (
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          <Clock className="w-12 h-12 mx-auto mb-4 opacity-50" />
          <p>No transcriptions yet</p>
          <p className="text-sm mt-1">
            Your transcription history will appear here
          </p>
        </div>
      )}

      {/* History list */}
      <div className="space-y-3">
        {entries.map((entry) => (
          <div
            key={entry.id}
            className="bg-white dark:bg-gray-800 rounded-lg shadow p-4 group"
          >
            {/* Text content */}
            <p className="text-sm text-gray-800 dark:text-gray-200 whitespace-pre-wrap break-words">
              {entry.text}
            </p>

            {/* Metadata and actions */}
            <div className="mt-3 flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
              <div className="flex items-center gap-3">
                {/* Play button */}
                {entry.audio_path && (
                  <button
                    onClick={() => toggleAudio(entry.id)}
                    disabled={loadingAudioId === entry.id}
                    className={`p-1.5 rounded transition-colors ${
                      playingId === entry.id
                        ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600'
                        : 'hover:bg-gray-100 dark:hover:bg-gray-700'
                    }`}
                    title={playingId === entry.id ? 'Stop' : 'Play audio'}
                  >
                    {loadingAudioId === entry.id ? (
                      <RefreshCw className="w-4 h-4 animate-spin" />
                    ) : playingId === entry.id ? (
                      <Square className="w-4 h-4" />
                    ) : (
                      <Play className="w-4 h-4" />
                    )}
                  </button>
                )}
                <span>{formatTimestamp(entry.timestamp)}</span>
                <span>{formatDuration(entry.duration_ms)}</span>
              </div>

              {/* Actions */}
              <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => copyToClipboard(entry.text, entry.id)}
                  className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
                  title="Copy to clipboard"
                >
                  {copiedId === entry.id ? (
                    <span className="text-green-500 text-xs">Copied!</span>
                  ) : (
                    <Copy className="w-4 h-4" />
                  )}
                </button>
                <button
                  onClick={() => deleteEntry(entry.id)}
                  className="p-1.5 hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 rounded transition-colors"
                  title="Delete"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Clear All Confirmation Modal */}
      {showClearConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50"
            onClick={() => setShowClearConfirm(false)}
          />
          {/* Modal */}
          <div className="relative bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-sm mx-4 animate-in fade-in zoom-in-95 duration-200">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-red-100 dark:bg-red-900/30 rounded-full">
                <AlertTriangle className="w-6 h-6 text-red-500" />
              </div>
              <h3 className="text-lg font-semibold">Clear All History</h3>
            </div>
            <p className="text-gray-600 dark:text-gray-400 mb-6">
              Are you sure you want to delete all transcriptions? This action cannot be undone.
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowClearConfirm(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={clearAllHistory}
                className="px-4 py-2 text-sm font-medium text-white bg-red-500 hover:bg-red-600 rounded-lg transition-colors"
              >
                Delete All
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
