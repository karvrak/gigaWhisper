import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { useSettings } from './useSettings';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core');

const mockSettings = {
  recording: {
    mode: 'push-to-talk' as const,
    max_duration: 300,
    silence_timeout: 0,
  },
  shortcuts: {
    record: 'Ctrl+Space',
    cancel: 'Escape',
    settings: 'Ctrl+Shift+W',
  },
  transcription: {
    provider: 'local' as const,
    language: 'auto',
    local: {
      model: 'small' as const,
      threads: 4,
      gpu_enabled: false,
    },
    groq: {
      api_key_configured: false,
      model: 'whisper-large-v3',
      timeout_seconds: 30,
    },
    openai: {
      api_key_configured: false,
      model: 'whisper-1',
      timeout_seconds: 30,
    },
    deepgram: {
      api_key_configured: false,
      model: 'nova-2',
      timeout_seconds: 30,
    },
  },
  audio: {
    input_device: null,
    vad: {
      enabled: false,
      aggressiveness: 2,
      min_speech_duration_ms: 250,
      padding_ms: 300,
    },
  },
  output: {
    auto_capitalize: true,
    auto_punctuation: true,
    paste_delay: 50,
  },
  ui: {
    show_indicator: true,
    indicator_position: 'cursor' as const,
    theme: 'system' as const,
    start_minimized: false,
    minimize_to_tray: true,
    auto_start: false,
    auto_update: false,
    custom_theme: null,
  },
  premium: {
    is_premium: false,
    expires_at: null,
    last_validated: null,
    credits: {
      balance_eur: 0,
      low_threshold_eur: 5,
    },
  },
  contexts: {
    active_context: 'default',
    contexts: [{
      id: 'default',
      name: 'Default',
      shortcut: '',
      language: 'auto',
      provider: 'local' as const,
      model: null,
      post_processing: null,
      color: null,
      icon: null,
      custom_vocabulary: null,
      app_patterns: [],
    }],
  },
  post_processing: {
    enabled: false,
    default_provider: 'groq-llm' as const,
    default_prompt: 'Clean up and fix any errors in this transcription.',
    remove_filler_words: false,
    openai: {
      api_key_configured: false,
      model: 'gpt-4o-mini',
    },
    anthropic: {
      api_key_configured: false,
      model: 'claude-haiku-4-5-20251001',
    },
    groq_llm: {
      model: 'llama-3.1-8b-instant',
    },
  },
};

describe('useSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it('should load settings on mount', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockSettings);

    const { result } = renderHook(() => useSettings());

    // Initially loading
    expect(result.current.loading).toBe(true);
    expect(result.current.settings).toBe(null);

    // Wait for settings to load
    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.settings).toEqual(mockSettings);
    expect(result.current.error).toBe(null);
    expect(invoke).toHaveBeenCalledWith('get_settings');
  });

  it('should handle load error', async () => {
    const errorMessage = 'Failed to load settings';
    vi.mocked(invoke).mockRejectedValueOnce(new Error(errorMessage));

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.settings).toBe(null);
    expect(result.current.error).toContain(errorMessage);
  });

  it('should update settings', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSettings) // Initial load
      .mockResolvedValueOnce(undefined); // Save

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const updatedSettings = {
      ...mockSettings,
      recording: {
        ...mockSettings.recording,
        mode: 'toggle' as const,
      },
    };

    act(() => {
      result.current.updateSettings(updatedSettings);
    });

    expect(result.current.settings?.recording.mode).toBe('toggle');

    // Wait for debounced save to complete
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('save_settings', { settings: updatedSettings });
    }, { timeout: 1000 });

    await waitFor(() => {
      expect(result.current.saving).toBe(false);
    });
  });

  it('should handle save error', async () => {
    const errorMessage = 'Failed to save settings';
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSettings) // Initial load
      .mockRejectedValueOnce(new Error(errorMessage)); // Save error

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const updatedSettings = {
      ...mockSettings,
      recording: {
        ...mockSettings.recording,
        mode: 'toggle' as const,
      },
    };

    act(() => {
      result.current.updateSettings(updatedSettings);
    });

    // Wait for debounced save to complete and error to be set
    await waitFor(() => {
      expect(result.current.error).toContain(errorMessage);
    }, { timeout: 1000 });

    expect(result.current.saving).toBe(false);
  });

  it('should reset settings', async () => {
    const defaultSettings = { ...mockSettings };
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSettings) // Initial load
      .mockResolvedValueOnce(defaultSettings); // Reset load

    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.resetSettings();
    });

    expect(result.current.settings).toEqual(defaultSettings);
    expect(invoke).toHaveBeenCalledTimes(2);
  });
});
