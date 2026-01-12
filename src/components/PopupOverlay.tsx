import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Check, Copy, X } from 'lucide-react';
import clsx from 'clsx';

interface PopupOverlayProps {
  onClose?: () => void;
}

export function PopupOverlay({ onClose }: PopupOverlayProps) {
  const [text, setText] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [visible, setVisible] = useState(false);

  // Listen for popup show events
  useEffect(() => {
    const unsubscribe = listen<string>('show:popup', (event) => {
      setText(event.payload);
      setVisible(true);
      setCopied(false);
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  // Auto-hide after 10 seconds
  useEffect(() => {
    if (visible) {
      const timer = setTimeout(() => {
        handleClose();
      }, 10000);
      return () => clearTimeout(timer);
    }
  }, [visible]);

  const handleClose = useCallback(() => {
    setVisible(false);
    setTimeout(() => {
      setText(null);
      onClose?.();
    }, 300); // Wait for animation
  }, [onClose]);

  const handleCopy = useCallback(async () => {
    if (text) {
      try {
        await writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (error) {
        console.error('Failed to copy:', error);
      }
    }
  }, [text]);

  // Handle escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && visible) {
        handleClose();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'c' && visible && text) {
        handleCopy();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [visible, text, handleClose, handleCopy]);

  if (!text) return null;

  return (
    <div
      className={clsx(
        'fixed inset-0 flex items-center justify-center z-50 transition-opacity duration-300',
        visible ? 'opacity-100' : 'opacity-0 pointer-events-none'
      )}
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/20 backdrop-blur-sm"
        onClick={handleClose}
      />

      {/* Popup */}
      <div
        className={clsx(
          'relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-lg w-full mx-4 transform transition-all duration-300',
          visible ? 'scale-100 translate-y-0' : 'scale-95 translate-y-4'
        )}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400">
            Transcription Result
          </h3>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors"
          >
            <X className="w-4 h-4 text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4">
          <p className="text-gray-900 dark:text-gray-100 text-base leading-relaxed select-all">
            {text}
          </p>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 rounded-b-xl">
          <span className="text-xs text-gray-500">
            Press Ctrl+C to copy, Esc to close
          </span>

          <button
            onClick={handleCopy}
            className={clsx(
              'flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-colors',
              copied
                ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                : 'bg-blue-600 hover:bg-blue-700 text-white'
            )}
          >
            {copied ? (
              <>
                <Check className="w-4 h-4" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="w-4 h-4" />
                Copy
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
