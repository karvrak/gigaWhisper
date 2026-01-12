import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import './RecordingIndicator.css';

type IndicatorState = 'recording' | 'processing';

export function RecordingIndicatorWindow() {
  const [duration, setDuration] = useState(0);
  const [state, setState] = useState<IndicatorState>('recording');
  const timerRef = useRef<number | null>(null);
  const startTimeRef = useRef<number>(Date.now());

  // Function to start/restart the timer
  const startTimer = () => {
    // Clear any existing timer
    if (timerRef.current) {
      clearInterval(timerRef.current);
    }
    // Reset start time and duration
    startTimeRef.current = Date.now();
    setDuration(0);
    // Start new timer
    timerRef.current = window.setInterval(() => {
      setDuration(Date.now() - startTimeRef.current);
    }, 100);
  };

  // Function to stop the timer
  const stopTimer = () => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
  };

  // Listen for state changes from backend
  useEffect(() => {
    // Listen for all state changes
    const unsubStateChanged = listen<string>('recording:state-changed', (event) => {
      const newState = event.payload;
      if (newState === 'recording') {
        // Restart timer when recording starts
        startTimer();
        setState('recording');
      } else if (newState === 'processing') {
        // Stop timer when processing
        stopTimer();
        setState('processing');
      }
    });

    // Also listen for the specific indicator event
    const unsubProcessing = listen('indicator:processing', () => {
      stopTimer();
      setState('processing');
    });

    return () => {
      unsubStateChanged.then((fn) => fn());
      unsubProcessing.then((fn) => fn());
      stopTimer();
    };
  }, []);

  const formatDuration = (ms: number) => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${minutes}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="recording-indicator-container">
      <div className="recording-indicator">
        {state === 'recording' ? (
          <>
            {/* Recording pulse */}
            <div className="recording-dot-container">
              <div className="recording-dot" />
              <div className="recording-dot-pulse" />
            </div>

            {/* Waveform animation */}
            <div className="waveform">
              {[...Array(12)].map((_, i) => (
                <div
                  key={i}
                  className="waveform-bar"
                  style={{
                    animationDelay: `${i * 0.05}s`,
                  }}
                />
              ))}
            </div>

            {/* Duration */}
            <div className="recording-duration">{formatDuration(duration)}</div>
          </>
        ) : (
          <>
            {/* Processing state - green theme */}
            <div className="processing-dot-container">
              <div className="processing-dot" />
              <div className="processing-dot-pulse" />
            </div>

            {/* Processing waveform - green animated bars */}
            <div className="waveform processing-waveform">
              {[...Array(12)].map((_, i) => (
                <div
                  key={i}
                  className="waveform-bar processing-bar"
                  style={{
                    animationDelay: `${i * 0.08}s`,
                  }}
                />
              ))}
            </div>

            <div className="processing-text">Processing</div>
          </>
        )}
      </div>
    </div>
  );
}
