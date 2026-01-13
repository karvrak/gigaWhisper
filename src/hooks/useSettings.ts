import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Settings types matching Rust structs
interface Settings {
  recording: {
    mode: 'push-to-talk' | 'toggle';
    max_duration: number;
    silence_timeout: number;
  };
  shortcuts: {
    record: string;
    cancel: string;
    settings: string;
  };
  transcription: {
    provider: 'local' | 'groq';
    language: string;
    local: {
      model: 'tiny' | 'base' | 'small' | 'medium' | 'large';
      threads: number;
      gpu_enabled: boolean;
    };
    groq: {
      api_key_configured: boolean;
      model: string;
      timeout_seconds: number;
    };
  };
  audio: {
    input_device: string | null;
  };
  output: {
    auto_capitalize: boolean;
    auto_punctuation: boolean;
    paste_delay: number;
  };
  ui: {
    show_indicator: boolean;
    indicator_position: 'cursor' | 'center' | 'corner';
    theme: 'system' | 'light' | 'dark';
    start_minimized: boolean;
    minimize_to_tray: boolean;
  };
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load settings on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const data = await invoke<Settings>('get_settings');
        setSettings(data);
        setError(null);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
    };

    loadSettings();
  }, []);

  // Save settings with debounce
  const updateSettings = useCallback(async (newSettings: Settings) => {
    setSettings(newSettings);
    setSaving(true);

    try {
      await invoke('save_settings', { settings: newSettings });
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, []);

  // Reset to defaults
  const resetSettings = useCallback(async () => {
    setLoading(true);
    try {
      // Get fresh default settings from backend
      const data = await invoke<Settings>('get_settings');
      setSettings(data);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    settings,
    loading,
    saving,
    error,
    updateSettings,
    resetSettings,
  };
}
