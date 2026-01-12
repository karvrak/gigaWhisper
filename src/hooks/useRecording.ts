import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface RecordingState {
  state: 'idle' | 'recording' | 'processing' | 'error';
  duration_ms?: number;
  error?: string;
}

export function useRecording() {
  const [state, setState] = useState<RecordingState>({ state: 'idle' });
  const timerRef = useRef<number | null>(null);
  const startTimeRef = useRef<number>(0);

  // Listen for state changes from backend (shortcuts, etc.)
  useEffect(() => {
    const unsubStateChanged = listen<string>('recording:state-changed', (event) => {
      const newState = event.payload;
      if (newState === 'recording') {
        startTimeRef.current = Date.now();
        setState({ state: 'recording', duration_ms: 0 });
      } else if (newState === 'processing') {
        setState({ state: 'processing' });
      } else if (newState === 'idle') {
        setState({ state: 'idle' });
      } else if (newState === 'error') {
        setState({ state: 'error', error: 'Unknown error' });
      }
    });

    // Also listen for specific error events
    const unsubError = listen<string>('transcription:error', (event) => {
      setState({ state: 'error', error: event.payload });
      // Reset to idle after showing error
      setTimeout(() => setState({ state: 'idle' }), 3000);
    });

    return () => {
      unsubStateChanged.then((fn) => fn());
      unsubError.then((fn) => fn());
    };
  }, []);

  // Update duration while recording
  useEffect(() => {
    if (state.state === 'recording') {
      timerRef.current = window.setInterval(() => {
        setState((prev) => ({
          ...prev,
          duration_ms: Date.now() - startTimeRef.current,
        }));
      }, 100);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, [state.state]);

  const startRecording = useCallback(async () => {
    try {
      await invoke('start_recording');
      // State will be updated via event listener
    } catch (error) {
      setState({ state: 'error', error: String(error) });
    }
  }, []);

  const stopRecording = useCallback(async () => {
    try {
      await invoke('stop_recording');
      // State will be updated via event listener
    } catch (error) {
      setState({ state: 'error', error: String(error) });
    }
  }, []);

  const cancelRecording = useCallback(async () => {
    try {
      await invoke('cancel_recording');
      setState({ state: 'idle' });
    } catch (error) {
      setState({ state: 'error', error: String(error) });
    }
  }, []);

  // Initial state fetch
  useEffect(() => {
    const fetchInitialState = async () => {
      try {
        const backendState = await invoke<RecordingState>('get_recording_state');
        setState(backendState);
      } catch (error) {
        console.error('Failed to get initial recording state:', error);
      }
    };
    fetchInitialState();
  }, []);

  return {
    state,
    startRecording,
    stopRecording,
    cancelRecording,
  };
}
