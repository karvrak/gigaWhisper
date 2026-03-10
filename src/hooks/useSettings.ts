import { useState, useEffect, useCallback, useRef } from 'react';
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
    provider: 'local' | 'groq' | 'openai' | 'deepgram' | 'custom';
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
    openai: {
      api_key_configured: boolean;
      model: string;
      timeout_seconds: number;
    };
    deepgram: {
      api_key_configured: boolean;
      model: string;
      timeout_seconds: number;
    };
    custom: {
      api_key_configured: boolean;
      api_url: string;
      model: string;
      timeout_seconds: number;
      auth_type: 'bearer' | 'x-api-key' | 'custom';
      custom_header_name: string;
      accept_invalid_certs: boolean;
    };
  };
  audio: {
    input_device: string | null;
    vad: {
      enabled: boolean;
      aggressiveness: number;
      min_speech_duration_ms: number;
      padding_ms: number;
    };
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
    auto_start: boolean;
    auto_update: boolean;
    custom_theme: string | null;
  };
  premium: {
    is_premium: boolean;
    expires_at: number | null;
    last_validated: number | null;
    credits: {
      balance_eur: number;
      low_threshold_eur: number;
    };
  };
  contexts: {
    active_context: string;
    contexts: Array<{
      id: string;
      name: string;
      shortcut: string;
      language: string;
      provider: 'local' | 'groq' | 'openai' | 'deepgram';
      model: string | null;
      post_processing: {
        enabled: boolean;
        llm_provider: 'open-ai' | 'anthropic' | 'groq-llm' | 'custom-llm';
        system_prompt: string;
      } | null;
      color: string | null;
      icon: string | null;
      custom_vocabulary: string | null;
      app_patterns: string[];
    }>;
  };
  post_processing: {
    enabled: boolean;
    default_provider: 'open-ai' | 'anthropic' | 'groq-llm' | 'custom-llm';
    default_prompt: string;
    remove_filler_words: boolean;
    openai: {
      api_key_configured: boolean;
      model: string;
    };
    anthropic: {
      api_key_configured: boolean;
      model: string;
    };
    groq_llm: {
      model: string;
    };
    custom_llm: {
      api_key_configured: boolean;
      api_url: string;
      model: string;
      timeout_seconds: number;
      auth_type: 'bearer' | 'x-api-key' | 'custom';
      custom_header_name: string;
      accept_invalid_certs: boolean;
    };
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

  const debounceTimer = useRef<ReturnType<typeof setTimeout>>();

  // Save settings with debounce
  const updateSettings = useCallback((newSettings: Settings) => {
    setSettings(newSettings);

    if (debounceTimer.current) clearTimeout(debounceTimer.current);

    debounceTimer.current = setTimeout(async () => {
      setSaving(true);
      try {
        await invoke('save_settings', { settings: newSettings });
        setError(null);
      } catch (err) {
        setError(String(err));
      } finally {
        setSaving(false);
      }
    }, 300);
  }, []);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
    };
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
