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
  },
  audio: {
    input_device: null,
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

    await act(async () => {
      await result.current.updateSettings(updatedSettings);
    });

    expect(result.current.settings?.recording.mode).toBe('toggle');
    expect(result.current.saving).toBe(false);
    expect(invoke).toHaveBeenCalledWith('save_settings', { settings: updatedSettings });
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

    await act(async () => {
      await result.current.updateSettings(updatedSettings);
    });

    expect(result.current.error).toContain(errorMessage);
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
