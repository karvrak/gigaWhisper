import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { Onboarding } from './Onboarding';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/event');

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

describe('Onboarding', () => {
  const mockOnComplete = vi.fn();
  let mockListenCallbacks: Map<string, (event: { payload: unknown }) => void>;

  beforeEach(() => {
    vi.clearAllMocks();
    mockListenCallbacks = new Map();

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') {
        return Promise.resolve(mockSettings);
      }
      if (cmd === 'save_settings') {
        return Promise.resolve(undefined);
      }
      if (cmd === 'download_model') {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    vi.mocked(listen).mockImplementation((eventName: string, callback: (event: { payload: unknown }) => void) => {
      mockListenCallbacks.set(eventName, callback);
      return Promise.resolve(() => {});
    });

    // Mock matchMedia for theme detection
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: query.includes('dark') ? false : true,
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

  afterEach(() => {
    // Clean up dark class from document
    document.documentElement.classList.remove('dark');
  });

  // ============================================
  // Step Display Tests
  // ============================================

  describe('Step 0: Theme Selection', () => {
    it('should display theme step as the first step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(screen.getByText('Choose Your Theme')).toBeInTheDocument();
      expect(screen.getByText(/Select how you want GigaWhisper to look/i)).toBeInTheDocument();
    });

    it('should display all three theme options', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    it('should have System theme selected by default', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const systemButton = screen.getByText('System').closest('button');
      expect(systemButton).toHaveClass('border-blue-500');
    });

    it('should allow selecting Light theme', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const lightButton = screen.getByText('Light').closest('button');
      fireEvent.click(lightButton!);

      expect(lightButton).toHaveClass('border-blue-500');
      expect(document.documentElement.classList.contains('dark')).toBe(false);
    });

    it('should allow selecting Dark theme', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const darkButton = screen.getByText('Dark').closest('button');
      fireEvent.click(darkButton!);

      expect(darkButton).toHaveClass('border-blue-500');
      expect(document.documentElement.classList.contains('dark')).toBe(true);
    });

    it('should apply System theme based on prefers-color-scheme', () => {
      // Mock dark mode preference
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        value: vi.fn().mockImplementation((query: string) => ({
          matches: query.includes('dark'),
          media: query,
          onchange: null,
          addListener: vi.fn(),
          removeListener: vi.fn(),
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      render(<Onboarding onComplete={mockOnComplete} />);

      // System theme with dark preference should add dark class
      expect(document.documentElement.classList.contains('dark')).toBe(true);
    });
  });

  describe('Step 1: How It Works', () => {
    it('should display how it works step after clicking Next', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next'));

      expect(screen.getByText('Voice to Text, Instantly')).toBeInTheDocument();
      expect(screen.getByText(/Press your shortcut key, speak/i)).toBeInTheDocument();
    });

    it('should show keyboard shortcut keys', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next'));

      expect(screen.getByText('Ctrl')).toBeInTheDocument();
      expect(screen.getByText('Shift')).toBeInTheDocument();
      expect(screen.getByText('Space')).toBeInTheDocument();
    });

    it('should show recording indicator', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next'));

      expect(screen.getByText('Recording...')).toBeInTheDocument();
    });

    it('should show transcribed text example', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next'));

      expect(screen.getByText('Hello, this is my transcribed text!')).toBeInTheDocument();
    });
  });

  describe('Step 2: Model Selection', () => {
    const goToModelStep = () => {
      fireEvent.click(screen.getByText('Next')); // Step 0 -> 1
      fireEvent.click(screen.getByText('Next')); // Step 1 -> 2
    };

    it('should display model selection step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      expect(screen.getByText('Choose Your Model')).toBeInTheDocument();
      expect(screen.getByText(/Select a transcription model/i)).toBeInTheDocument();
    });

    it('should display all available models', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      expect(screen.getByText('Tiny')).toBeInTheDocument();
      expect(screen.getByText('Base')).toBeInTheDocument();
      expect(screen.getByText('Small')).toBeInTheDocument();
      expect(screen.getByText('Medium')).toBeInTheDocument();
    });

    it('should show model sizes and descriptions', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      expect(screen.getByText(/75 MB/i)).toBeInTheDocument();
      expect(screen.getByText(/142 MB/i)).toBeInTheDocument();
      expect(screen.getByText(/466 MB/i)).toBeInTheDocument();
      expect(screen.getByText(/1.5 GB/i)).toBeInTheDocument();
    });

    it('should mark Small model as recommended', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      expect(screen.getByText('Recommended')).toBeInTheDocument();
    });

    it('should have Small model selected by default', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const smallButton = screen.getByText('Small').closest('button');
      expect(smallButton).toHaveClass('border-blue-500');
    });

    it('should allow selecting a different model', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const baseButton = screen.getByText('Base').closest('button');
      fireEvent.click(baseButton!);

      expect(baseButton).toHaveClass('border-blue-500');
    });

    it('should show download button for selected model', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      expect(screen.getByText(/Download Small Model/i)).toBeInTheDocument();
    });

    it('should start download when download button is clicked', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('download_model', { model: 'small' });
      });
    });

    it('should show downloading progress', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      await waitFor(() => {
        expect(screen.getByText(/Downloading Small/i)).toBeInTheDocument();
      });
    });

    it('should update progress when receiving progress events', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      // Simulate progress event
      await waitFor(() => {
        const progressCallback = mockListenCallbacks.get('model-download-progress');
        expect(progressCallback).toBeDefined();
      });

      act(() => {
        const progressCallback = mockListenCallbacks.get('model-download-progress');
        if (progressCallback) {
          progressCallback({ payload: { model: 'small', percentage: 50 } });
        }
      });

      await waitFor(() => {
        expect(screen.getByText('50%')).toBeInTheDocument();
      });
    });

    it('should show completion status when download completes', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      // Simulate completion event
      await waitFor(() => {
        const completeCallback = mockListenCallbacks.get('model-download-complete');
        expect(completeCallback).toBeDefined();
      });

      act(() => {
        const completeCallback = mockListenCallbacks.get('model-download-complete');
        if (completeCallback) {
          completeCallback({ payload: { model: 'small' } });
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Model ready!')).toBeInTheDocument();
      });
    });

    it('should disable model selection during download', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      await waitFor(() => {
        const baseButton = screen.getByText('Base').closest('button');
        expect(baseButton).toBeDisabled();
      });
    });

    it('should reset download complete status when selecting different model', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      // Start and complete download
      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      await waitFor(() => {
        const completeCallback = mockListenCallbacks.get('model-download-complete');
        expect(completeCallback).toBeDefined();
      });

      act(() => {
        const completeCallback = mockListenCallbacks.get('model-download-complete');
        if (completeCallback) {
          completeCallback({ payload: { model: 'small' } });
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Model ready!')).toBeInTheDocument();
      });

      // Select different model
      const baseButton = screen.getByText('Base').closest('button');
      fireEvent.click(baseButton!);

      await waitFor(() => {
        expect(screen.queryByText('Model ready!')).not.toBeInTheDocument();
        expect(screen.getByText(/Download Base Model/i)).toBeInTheDocument();
      });
    });

    it('should handle download error gracefully', async () => {
      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === 'download_model') {
          return Promise.reject(new Error('Download failed'));
        }
        return Promise.resolve(mockSettings);
      });

      // Spy on console.error
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      render(<Onboarding onComplete={mockOnComplete} />);
      goToModelStep();

      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Download failed:', expect.any(Error));
      });

      // Should show download button again after error
      await waitFor(() => {
        expect(screen.getByText(/Download Small Model/i)).toBeInTheDocument();
      });

      consoleSpy.mockRestore();
    });
  });

  describe('Step 3: Ready to Go', () => {
    const goToReadyStep = () => {
      fireEvent.click(screen.getByText('Next')); // Step 0 -> 1
      fireEvent.click(screen.getByText('Next')); // Step 1 -> 2
      fireEvent.click(screen.getByText('Next')); // Step 2 -> 3
    };

    it('should display ready step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToReadyStep();

      expect(screen.getByText('Ready to Go!')).toBeInTheDocument();
      expect(screen.getByText(/GigaWhisper runs in your system tray/i)).toBeInTheDocument();
    });

    it('should show tips about customizing shortcut', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToReadyStep();

      expect(screen.getByText('Customize your shortcut')).toBeInTheDocument();
      expect(screen.getByText(/Settings/i)).toBeInTheDocument();
    });

    it('should show tips about switching modes', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToReadyStep();

      expect(screen.getByText('Switch modes')).toBeInTheDocument();
      expect(screen.getByText(/Push-to-talk or Toggle mode/i)).toBeInTheDocument();
    });

    it('should show Get Started button instead of Next', () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      goToReadyStep();

      expect(screen.getByText('Get Started')).toBeInTheDocument();
      expect(screen.queryByText('Next')).not.toBeInTheDocument();
    });
  });

  // ============================================
  // Navigation Tests
  // ============================================

  describe('Navigation', () => {
    it('should show Next button on first step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(screen.getByText('Next')).toBeInTheDocument();
    });

    it('should not show Back button on first step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(screen.queryByText('Back')).not.toBeInTheDocument();
    });

    it('should show Back button after first step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next'));

      expect(screen.getByText('Back')).toBeInTheDocument();
    });

    it('should navigate back when Back is clicked', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next')); // Go to step 1
      expect(screen.getByText('Voice to Text, Instantly')).toBeInTheDocument();

      fireEvent.click(screen.getByText('Back')); // Go back to step 0
      expect(screen.getByText('Choose Your Theme')).toBeInTheDocument();
    });

    it('should display step indicators', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Should have 4 step indicators (buttons)
      const stepButtons = screen.getAllByRole('button').filter(
        (button) => button.classList.contains('rounded-full') && button.classList.contains('h-2')
      );
      expect(stepButtons.length).toBe(4);
    });

    it('should highlight current step indicator', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const stepButtons = screen.getAllByRole('button').filter(
        (button) => button.classList.contains('rounded-full') && button.classList.contains('h-2')
      );

      // First step should be wider (active)
      expect(stepButtons[0]).toHaveClass('w-6');
      expect(stepButtons[1]).toHaveClass('w-2');
    });

    it('should allow clicking step indicators to navigate', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const stepButtons = screen.getAllByRole('button').filter(
        (button) => button.classList.contains('rounded-full') && button.classList.contains('h-2')
      );

      // Click on step 3 (Ready)
      fireEvent.click(stepButtons[3]);

      expect(screen.getByText('Ready to Go!')).toBeInTheDocument();
    });

    it('should update step indicator when navigating', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      fireEvent.click(screen.getByText('Next')); // Go to step 1

      const stepButtons = screen.getAllByRole('button').filter(
        (button) => button.classList.contains('rounded-full') && button.classList.contains('h-2')
      );

      // Second step should now be wider (active)
      expect(stepButtons[0]).toHaveClass('w-2');
      expect(stepButtons[1]).toHaveClass('w-6');
    });
  });

  // ============================================
  // Completion and Settings Tests
  // ============================================

  describe('Completion and Settings', () => {
    const completeOnboarding = () => {
      fireEvent.click(screen.getByText('Next')); // Step 0 -> 1
      fireEvent.click(screen.getByText('Next')); // Step 1 -> 2
      fireEvent.click(screen.getByText('Next')); // Step 2 -> 3
      fireEvent.click(screen.getByText('Get Started')); // Complete
    };

    it('should call onComplete when Get Started is clicked', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      completeOnboarding();

      await waitFor(() => {
        expect(mockOnComplete).toHaveBeenCalled();
      });
    });

    it('should save settings with selected theme', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Select dark theme
      const darkButton = screen.getByText('Dark').closest('button');
      fireEvent.click(darkButton!);

      completeOnboarding();

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('save_settings', {
          settings: expect.objectContaining({
            ui: expect.objectContaining({
              theme: 'dark',
            }),
          }),
        });
      });
    });

    it('should save settings with selected model', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Go to model step and select Base
      fireEvent.click(screen.getByText('Next')); // Step 0 -> 1
      fireEvent.click(screen.getByText('Next')); // Step 1 -> 2

      const baseButton = screen.getByText('Base').closest('button');
      fireEvent.click(baseButton!);

      fireEvent.click(screen.getByText('Next')); // Step 2 -> 3
      fireEvent.click(screen.getByText('Get Started')); // Complete

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('save_settings', {
          settings: expect.objectContaining({
            transcription: expect.objectContaining({
              local: expect.objectContaining({
                model: 'base',
              }),
            }),
          }),
        });
      });
    });

    it('should get current settings before saving', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      completeOnboarding();

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_settings');
      });
    });

    it('should preserve existing settings when saving', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);
      completeOnboarding();

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('save_settings', {
          settings: expect.objectContaining({
            shortcuts: mockSettings.shortcuts,
            recording: mockSettings.recording,
          }),
        });
      });
    });

    it('should handle save settings error gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      vi.mocked(invoke).mockImplementation((cmd: string) => {
        if (cmd === 'get_settings') {
          return Promise.resolve(mockSettings);
        }
        if (cmd === 'save_settings') {
          return Promise.reject(new Error('Save failed'));
        }
        return Promise.resolve(undefined);
      });

      render(<Onboarding onComplete={mockOnComplete} />);
      completeOnboarding();

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Failed to save settings:', expect.any(Error));
      });

      // onComplete should still be called even if save fails
      expect(mockOnComplete).toHaveBeenCalled();

      consoleSpy.mockRestore();
    });
  });

  // ============================================
  // Event Listener Cleanup Tests
  // ============================================

  describe('Event Listener Cleanup', () => {
    it('should set up event listeners on mount', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(listen).toHaveBeenCalledWith('model-download-progress', expect.any(Function));
      expect(listen).toHaveBeenCalledWith('model-download-complete', expect.any(Function));
      expect(listen).toHaveBeenCalledWith('model-download-error', expect.any(Function));
    });

    it('should clean up event listeners on unmount', () => {
      const unlistenMocks = [vi.fn(), vi.fn(), vi.fn()];
      let callIndex = 0;

      vi.mocked(listen).mockImplementation(() => {
        const unlisten = unlistenMocks[callIndex];
        callIndex++;
        return Promise.resolve(unlisten);
      });

      const { unmount } = render(<Onboarding onComplete={mockOnComplete} />);
      unmount();

      // Wait for cleanup
      return new Promise<void>((resolve) => {
        setTimeout(() => {
          unlistenMocks.forEach((mock) => {
            expect(mock).toHaveBeenCalled();
          });
          resolve();
        }, 100);
      });
    });
  });

  // ============================================
  // UI Structure Tests
  // ============================================

  describe('UI Structure', () => {
    it('should render overlay backdrop', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const overlay = document.querySelector('.fixed.inset-0.z-50');
      expect(overlay).toBeInTheDocument();
      expect(overlay).toHaveClass('bg-gray-900/50');
    });

    it('should render modal container', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const modal = document.querySelector('.bg-white.dark\\:bg-gray-800.rounded-2xl');
      expect(modal).toBeInTheDocument();
    });

    it('should have proper max-width for modal', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const modal = document.querySelector('.max-w-lg');
      expect(modal).toBeInTheDocument();
    });
  });

  // ============================================
  // Accessibility Tests
  // ============================================

  describe('Accessibility', () => {
    it('should have descriptive headings for each step', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      expect(screen.getByRole('heading', { level: 2, name: 'Choose Your Theme' })).toBeInTheDocument();

      fireEvent.click(screen.getByText('Next'));
      expect(screen.getByRole('heading', { level: 2, name: 'Voice to Text, Instantly' })).toBeInTheDocument();
    });

    it('should have clickable navigation buttons', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      const nextButton = screen.getByText('Next').closest('button');
      expect(nextButton).toBeEnabled();
    });
  });

  // ============================================
  // Edge Cases
  // ============================================

  describe('Edge Cases', () => {
    it('should handle rapid navigation clicks', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Rapid clicks
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Next'));

      // Should be at step 3
      expect(screen.getByText('Ready to Go!')).toBeInTheDocument();
    });

    it('should not go past last step with Next', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Go to last step
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Next'));

      // "Get Started" should complete, not navigate
      expect(screen.getByText('Ready to Go!')).toBeInTheDocument();
    });

    it('should not go before first step with Back', () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Back button should not exist on first step
      expect(screen.queryByText('Back')).not.toBeInTheDocument();

      // Navigate forward then back
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Back'));

      // Should be at first step
      expect(screen.getByText('Choose Your Theme')).toBeInTheDocument();
      expect(screen.queryByText('Back')).not.toBeInTheDocument();
    });

    it('should handle download error event', async () => {
      render(<Onboarding onComplete={mockOnComplete} />);

      // Go to model step
      fireEvent.click(screen.getByText('Next'));
      fireEvent.click(screen.getByText('Next'));

      // Start download
      const downloadButton = screen.getByText(/Download Small Model/i);
      fireEvent.click(downloadButton);

      // Wait for listeners to be set up
      await waitFor(() => {
        const errorCallback = mockListenCallbacks.get('model-download-error');
        expect(errorCallback).toBeDefined();
      });

      // Simulate error event
      act(() => {
        const errorCallback = mockListenCallbacks.get('model-download-error');
        if (errorCallback) {
          errorCallback({ payload: { model: 'small', error: 'Network error' } });
        }
      });

      // Download button should reappear
      await waitFor(() => {
        expect(screen.getByText(/Download Small Model/i)).toBeInTheDocument();
      });
    });
  });
});
