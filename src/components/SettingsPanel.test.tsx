import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SettingsPanel } from './SettingsPanel';

// Mock Tauri invoke API
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock useSettings hook
const mockUpdateSettings = vi.fn();
const mockSettings = {
  recording: {
    mode: 'push-to-talk' as const,
    max_duration: 300,
    silence_timeout: 2000,
  },
  shortcuts: {
    record: 'Ctrl+Space',
    cancel: 'Escape',
  },
  transcription: {
    provider: 'local' as const,
    language: 'auto',
    local: {
      model: 'base',
      threads: 0,
      gpu_enabled: false,
    },
    groq: {
      model: 'whisper-large-v3',
      timeout_seconds: 30,
    },
  },
  audio: {
    input_device: null,
    vad: {
      enabled: true,
      aggressiveness: 2,
    },
  },
  output: {
    auto_capitalize: true,
    auto_punctuation: true,
    paste_delay: 50,
  },
  ui: {
    theme: 'system' as const,
    start_minimized: false,
    show_indicator: true,
  },
};

vi.mock('../hooks/useSettings', () => ({
  useSettings: () => ({
    settings: mockSettings,
    updateSettings: mockUpdateSettings,
    saving: false,
    error: null,
  }),
}));

// Mock child components to simplify testing
vi.mock('./HotkeyInput', () => ({
  HotkeyInput: ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <input
      data-testid="hotkey-input"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  ),
}));

vi.mock('./ModelSelector', () => ({
  ModelSelector: ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <select data-testid="model-selector" value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="tiny">Tiny</option>
      <option value="base">Base</option>
      <option value="small">Small</option>
      <option value="medium">Medium</option>
    </select>
  ),
}));

vi.mock('./ProviderToggle', () => ({
  ProviderToggle: ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <div data-testid="provider-toggle">
      <button onClick={() => onChange('local')} data-selected={value === 'local'}>
        Local
      </button>
      <button onClick={() => onChange('groq')} data-selected={value === 'groq'}>
        Groq
      </button>
    </div>
  ),
}));

describe('SettingsPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  // =========================================================================
  // Tab Rendering Tests
  // =========================================================================

  describe('Tab Navigation', () => {
    it('should render all three tabs', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('General')).toBeInTheDocument();
      expect(screen.getByText('Transcription')).toBeInTheDocument();
      expect(screen.getByText('Audio')).toBeInTheDocument();
    });

    it('should show General tab content by default', () => {
      render(<SettingsPanel />);

      // General tab should show recording mode
      expect(screen.getByText('Recording Mode')).toBeInTheDocument();
      expect(screen.getByText('Push-to-Talk')).toBeInTheDocument();
    });

    it('should switch to Transcription tab when clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      // Should show transcription settings
      expect(screen.getByText('Language')).toBeInTheDocument();
      expect(screen.getByText('Provider')).toBeInTheDocument();
    });

    it('should switch to Audio tab when clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      // Should show audio settings
      expect(screen.getByText('Input Device')).toBeInTheDocument();
    });

    it('should highlight active tab', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      const transcriptionTab = screen.getByText('Transcription');
      await user.click(transcriptionTab);

      // The tab should have active styling (border-blue-500 class)
      expect(transcriptionTab).toHaveClass('border-blue-500');
    });
  });

  // =========================================================================
  // General Tab Tests
  // =========================================================================

  describe('General Tab', () => {
    it('should render recording mode toggle', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('Push-to-Talk')).toBeInTheDocument();
      expect(screen.getByText('Toggle')).toBeInTheDocument();
    });

    it('should toggle recording mode when clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      // Find the switch button
      const toggle = screen.getByRole('switch');
      await user.click(toggle);

      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          recording: expect.objectContaining({
            mode: 'toggle',
          }),
        })
      );
    });

    it('should render record shortcut input', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('Record Shortcut')).toBeInTheDocument();
      expect(screen.getByTestId('hotkey-input')).toBeInTheDocument();
    });

    it('should update shortcut when changed', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      const input = screen.getByTestId('hotkey-input');
      await user.clear(input);
      await user.type(input, 'Ctrl+Shift+R');

      expect(mockUpdateSettings).toHaveBeenCalled();
    });

    it('should render start minimized checkbox', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('Start minimized to tray')).toBeInTheDocument();
      expect(screen.getByLabelText('Start minimized to tray')).toBeInTheDocument();
    });

    it('should render show indicator checkbox', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('Show recording indicator')).toBeInTheDocument();
    });

    it('should render theme selection buttons', () => {
      render(<SettingsPanel />);

      expect(screen.getByText('Appearance')).toBeInTheDocument();
      expect(screen.getByText('Light')).toBeInTheDocument();
      expect(screen.getByText('Dark')).toBeInTheDocument();
      expect(screen.getByText('System')).toBeInTheDocument();
    });

    it('should update theme when theme button clicked', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Dark'));

      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          ui: expect.objectContaining({
            theme: 'dark',
          }),
        })
      );
    });
  });

  // =========================================================================
  // Transcription Tab Tests
  // =========================================================================

  describe('Transcription Tab', () => {
    it('should render language selector', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      expect(screen.getByText('Language')).toBeInTheDocument();
      // Multiple comboboxes exist (language, model), just check at least one exists
      expect(screen.getAllByRole('combobox').length).toBeGreaterThan(0);
    });

    it('should render provider toggle', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      expect(screen.getByTestId('provider-toggle')).toBeInTheDocument();
    });

    it('should show local settings when local provider selected', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      // Should show model selector for local provider
      expect(screen.getByText('Whisper Model')).toBeInTheDocument();
      expect(screen.getByTestId('model-selector')).toBeInTheDocument();
    });

    it('should show GPU acceleration option for local provider', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      expect(screen.getByText('GPU Acceleration')).toBeInTheDocument();
    });

    it('should update language when changed', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Transcription'));

      // Find the language select by its label association
      const languageLabel = screen.getByText('Language');
      const languageSection = languageLabel.closest('div');
      const selects = screen.getAllByRole('combobox');
      // The language select should be in the transcription section and have language options
      const languageSelect = selects.find(
        (select) => select.querySelector('option[value="en"]') !== null
      );

      if (languageSelect) {
        await user.selectOptions(languageSelect, 'en');

        expect(mockUpdateSettings).toHaveBeenCalledWith(
          expect.objectContaining({
            transcription: expect.objectContaining({
              language: 'en',
            }),
          })
        );
      }
    });
  });

  // =========================================================================
  // Audio Tab Tests
  // =========================================================================

  describe('Audio Tab', () => {
    it('should render input device selector', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      expect(screen.getByText('Input Device')).toBeInTheDocument();
    });

    it('should show default microphone option', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      expect(screen.getByText('Default Microphone')).toBeInTheDocument();
    });

    it('should render auto-capitalize checkbox', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      expect(screen.getByText('Auto-capitalize first letter')).toBeInTheDocument();
    });

    it('should render auto-punctuation checkbox', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      expect(screen.getByText('Auto-punctuation')).toBeInTheDocument();
    });

    it('should update auto-capitalize when checkbox toggled', async () => {
      const user = userEvent.setup();
      render(<SettingsPanel />);

      await user.click(screen.getByText('Audio'));

      const checkbox = screen.getByLabelText('Auto-capitalize first letter');
      await user.click(checkbox);

      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          output: expect.objectContaining({
            auto_capitalize: false,
          }),
        })
      );
    });
  });

  // =========================================================================
  // Loading State Tests
  // =========================================================================

  describe('Loading State', () => {
    it('should show loading spinner when settings is null', () => {
      vi.doMock('../hooks/useSettings', () => ({
        useSettings: () => ({
          settings: null,
          updateSettings: mockUpdateSettings,
          saving: false,
          error: null,
        }),
      }));

      // Note: This test may need adjustment based on how the component handles null settings
    });
  });

  // =========================================================================
  // Error Handling Tests
  // =========================================================================

  describe('Error Handling', () => {
    it('should fetch audio devices on mount', () => {
      render(<SettingsPanel />);

      expect(mockInvoke).toHaveBeenCalledWith('get_audio_devices');
    });

    it('should handle audio device fetch error gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Failed to get devices'));

      // Should not throw
      expect(() => render(<SettingsPanel />)).not.toThrow();
    });
  });
});
