import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import App from '../App';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event');
vi.mock('@tauri-apps/api/window');

const mockSettings = {
  recording: {
    mode: 'push-to-talk' as const,
    max_duration: 300,
    silence_timeout: 0,
  },
  shortcuts: {
    record: 'Ctrl+Shift+Space',
    cancel: 'Escape',
    settings: 'Ctrl+Shift+W',
  },
  transcription: {
    provider: 'local' as const,
    language: 'auto',
    local: {
      model: 'base' as const,
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

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Clear localStorage
    localStorage.clear();

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') {
        return Promise.resolve(mockSettings);
      }
      return Promise.resolve(undefined);
    });

    vi.mocked(listen).mockImplementation(() => {
      return Promise.resolve(() => {});
    });

    vi.mocked(getCurrentWindow).mockReturnValue({
      minimize: vi.fn().mockResolvedValue(undefined),
      hide: vi.fn().mockResolvedValue(undefined),
    } as any);

    // Mock matchMedia
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
  });

  it('should show loading state initially', () => {
    render(<App />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should render main content after loading', async () => {
    // Mark onboarding as completed
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Keyboard Shortcut')).toBeInTheDocument();
    });
  });

  it('should display shortcut from settings', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Ctrl+Shift+Space')).toBeInTheDocument();
    });
  });

  it('should render navigation tabs', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Home')).toBeInTheDocument();
      expect(screen.getByText('History')).toBeInTheDocument();
      expect(screen.getByText('Settings')).toBeInTheDocument();
    });
  });

  it('should switch to Settings view when Settings tab is clicked', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Home')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Settings'));

    await waitFor(() => {
      expect(screen.getByText('Recording Mode')).toBeInTheDocument();
    });
  });

  it('should switch to History view when History tab is clicked', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') {
        return Promise.resolve(mockSettings);
      }
      if (cmd === 'get_transcription_history') {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText('Home')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('History'));

    await waitFor(() => {
      expect(screen.getByText('Transcription History')).toBeInTheDocument();
    });
  });

  it('should show push-to-talk description when in PTT mode', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/Hold the shortcut to record/i)).toBeInTheDocument();
    });
  });

  it('should show toggle description when in toggle mode', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    const toggleSettings = {
      ...mockSettings,
      recording: { ...mockSettings.recording, mode: 'toggle' as const },
    };

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') {
        return Promise.resolve(toggleSettings);
      }
      return Promise.resolve(undefined);
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/Press once to start/i)).toBeInTheDocument();
    });
  });

  it('should show local provider info when local is selected', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/Local - Whisper base/i)).toBeInTheDocument();
    });
  });

  it('should show Groq provider info when groq is selected', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    const groqSettings = {
      ...mockSettings,
      transcription: { ...mockSettings.transcription, provider: 'groq' as const },
    };

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') {
        return Promise.resolve(groqSettings);
      }
      return Promise.resolve(undefined);
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText(/Cloud - Groq API/i)).toBeInTheDocument();
    });
  });

  it('should call minimize when minimize button is clicked', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByTitle('Minimize')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTitle('Minimize'));

    const mockWindow = getCurrentWindow();
    expect(mockWindow.minimize).toHaveBeenCalled();
  });

  it('should call hide when close button is clicked', async () => {
    localStorage.setItem('gigawhisper_onboarding_completed', 'true');

    render(<App />);

    await waitFor(() => {
      expect(screen.getByTitle('Close')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTitle('Close'));

    const mockWindow = getCurrentWindow();
    expect(mockWindow.hide).toHaveBeenCalled();
  });
});
