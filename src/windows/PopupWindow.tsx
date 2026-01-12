import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { Copy, Check, X } from 'lucide-react';
import './PopupWindow.css';

// Helper function to apply theme
function applyTheme(theme: 'system' | 'light' | 'dark') {
  const root = document.documentElement;
  if (theme === 'dark') {
    root.classList.add('dark');
  } else if (theme === 'light') {
    root.classList.remove('dark');
  } else {
    // System preference
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }
}

interface Settings {
  ui: {
    theme: 'system' | 'light' | 'dark';
  };
}

export function PopupWindow() {
  const [text, setText] = useState('');
  const [copied, setCopied] = useState(false);

  // Load and apply theme on mount
  useEffect(() => {
    const loadTheme = async () => {
      try {
        const settings = await invoke<Settings>('get_settings');
        if (settings?.ui?.theme) {
          applyTheme(settings.ui.theme);
        }
      } catch (e) {
        console.error('Failed to load settings:', e);
      }
    };
    loadTheme();
  }, []);

  // Listen for popup show event
  useEffect(() => {
    const unsubscribe = listen<string>('show:popup', (event) => {
      setText(event.payload);
      setCopied(false);
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  };

  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.hide();
  };

  if (!text) {
    return (
      <div className="popup-container">
        <div className="popup-empty">
          Waiting for transcription...
        </div>
      </div>
    );
  }

  return (
    <div className="popup-container">
      <div className="popup-window">
        {/* Header */}
        <div className="popup-header">
          <span className="popup-title">Transcription</span>
          <button
            onClick={handleClose}
            className="popup-close-btn"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="popup-content">
          <p className="popup-text">{text}</p>
        </div>

        {/* Footer */}
        <div className="popup-footer">
          <button
            onClick={handleCopy}
            className={`popup-copy-btn ${copied ? 'copied' : ''}`}
          >
            {copied ? (
              <>
                <Check className="w-4 h-4" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="w-4 h-4" />
                Copy to Clipboard
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
