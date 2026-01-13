import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useRecording } from './useRecording';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type EventCallback = (event: any) => void;
type UnsubscribeFn = () => void;

describe('useRecording', () => {
  let stateChangedCallback: EventCallback | null = null;
  let errorCallback: EventCallback | null = null;
  let micErrorCallback: EventCallback | null = null;
  let unsubscribeFns: UnsubscribeFn[] = [];

  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    stateChangedCallback = null;
    errorCallback = null;
    micErrorCallback = null;
    unsubscribeFns = [];

    // Mock listen to capture callbacks
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    vi.mocked(listen).mockImplementation((eventName: any, callback: any) => {
      if (eventName === 'recording:state-changed') {
        stateChangedCallback = callback;
      } else if (eventName === 'transcription:error') {
        errorCallback = callback;
      } else if (eventName === 'recording:microphone-error') {
        micErrorCallback = callback;
      }
      const unsub = () => {};
      unsubscribeFns.push(unsub);
      return Promise.resolve(unsub);
    });

    // Mock initial state fetch
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_recording_state') {
        return Promise.resolve({ state: 'idle' });
      }
      return Promise.resolve(undefined);
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('should initialize with idle state', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(result.current.state.state).toBe('idle');
    });

    unmount();
  });

  it('should start recording', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(result.current.state.state).toBe('idle');
    });

    await act(async () => {
      await result.current.startRecording();
    });

    expect(invoke).toHaveBeenCalledWith('start_recording');

    unmount();
  });

  it('should stop recording', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(result.current.state.state).toBe('idle');
    });

    await act(async () => {
      await result.current.stopRecording();
    });

    expect(invoke).toHaveBeenCalledWith('stop_recording');

    unmount();
  });

  it('should cancel recording', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(result.current.state.state).toBe('idle');
    });

    await act(async () => {
      await result.current.cancelRecording();
    });

    expect(invoke).toHaveBeenCalledWith('cancel_recording');
    expect(result.current.state.state).toBe('idle');

    unmount();
  });

  it('should handle state changed event to recording', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(stateChangedCallback).not.toBe(null);
    });

    act(() => {
      stateChangedCallback?.({ payload: 'recording' });
    });

    expect(result.current.state.state).toBe('recording');
    expect(result.current.state.duration_ms).toBe(0);

    // Stop the recording to cleanup interval
    act(() => {
      stateChangedCallback?.({ payload: 'idle' });
    });

    unmount();
  });

  it('should handle state changed event to processing', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(stateChangedCallback).not.toBe(null);
    });

    act(() => {
      stateChangedCallback?.({ payload: 'processing' });
    });

    expect(result.current.state.state).toBe('processing');

    unmount();
  });

  it('should handle state changed event to idle', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(stateChangedCallback).not.toBe(null);
    });

    // First set to recording
    act(() => {
      stateChangedCallback?.({ payload: 'recording' });
    });

    expect(result.current.state.state).toBe('recording');

    // Then back to idle
    act(() => {
      stateChangedCallback?.({ payload: 'idle' });
    });

    expect(result.current.state.state).toBe('idle');

    unmount();
  });

  it('should handle error event and set error state', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(errorCallback).not.toBe(null);
    });

    act(() => {
      errorCallback?.({ payload: 'Test error message' });
    });

    expect(result.current.state.state).toBe('error');
    expect(result.current.state.error).toBe('Test error message');

    // Advance time to allow the setTimeout cleanup
    act(() => {
      vi.advanceTimersByTime(3500);
    });

    unmount();
  });

  it('should handle start recording error', async () => {
    const errorMessage = 'Microphone not available';
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_recording_state') {
        return Promise.resolve({ state: 'idle' });
      }
      if (cmd === 'start_recording') {
        return Promise.reject(new Error(errorMessage));
      }
      return Promise.resolve(undefined);
    });

    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(result.current.state.state).toBe('idle');
    });

    await act(async () => {
      await result.current.startRecording();
    });

    expect(result.current.state.state).toBe('error');
    expect(result.current.state.error).toContain(errorMessage);

    unmount();
  });

  it('should handle state changed event to error', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(stateChangedCallback).not.toBe(null);
    });

    act(() => {
      stateChangedCallback?.({ payload: 'error' });
    });

    expect(result.current.state.state).toBe('error');
    expect(result.current.state.error).toBe('Unknown error');

    unmount();
  });

  it('should handle microphone disconnection error', async () => {
    const { result, unmount } = renderHook(() => useRecording());

    await vi.waitFor(() => {
      expect(micErrorCallback).not.toBe(null);
    });

    act(() => {
      micErrorCallback?.({ payload: 'Microphone disconnected during recording' });
    });

    expect(result.current.state.state).toBe('error');
    expect(result.current.state.error).toBe('Microphone disconnected during recording');

    // Should reset to idle after 5 seconds (longer for hardware issues)
    act(() => {
      vi.advanceTimersByTime(5500);
    });

    unmount();
  });
});
