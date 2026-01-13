import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from '../hooks/useSettings';
import { HotkeyInput } from './HotkeyInput';
import { ModelSelector } from './ModelSelector';
import { ProviderToggle } from './ProviderToggle';
import { Sun, Moon, Monitor, Eye, EyeOff, Check, X, Loader2 } from 'lucide-react';

// Separate component for Groq API Key management (uses secure storage)
function GroqApiKeyInput() {
  const [apiKey, setApiKey] = useState('');
  const [hasKey, setHasKey] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check if API key is already configured
  useEffect(() => {
    const checkApiKey = async () => {
      try {
        const exists = await invoke<boolean>('has_groq_api_key');
        setHasKey(exists);
      } catch (e) {
        console.error('Failed to check API key:', e);
      }
    };
    checkApiKey();
  }, []);

  const handleSave = useCallback(async () => {
    if (!apiKey.trim()) {
      setError('API key cannot be empty');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      // Validate the key format first
      await invoke('validate_groq_api_key', { apiKey: apiKey.trim() });
      // Save the key securely
      await invoke('set_groq_api_key', { apiKey: apiKey.trim() });
      setHasKey(true);
      setIsEditing(false);
      setApiKey('');
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, [apiKey]);

  const handleClear = useCallback(async () => {
    setSaving(true);
    setError(null);

    try {
      await invoke('clear_groq_api_key');
      setHasKey(false);
      setApiKey('');
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }, []);

  const handleCancel = useCallback(() => {
    setIsEditing(false);
    setApiKey('');
    setError(null);
  }, []);

  if (hasKey && !isEditing) {
    return (
      <div>
        <label className="block text-sm font-medium mb-2">Groq API Key</label>
        <div className="flex items-center gap-2">
          <div className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-gray-50 dark:bg-gray-700 text-gray-500 dark:text-gray-400 font-mono text-sm">
            ****************************************
          </div>
          <button
            onClick={() => setIsEditing(true)}
            className="px-3 py-2 text-sm text-blue-600 hover:text-blue-700 border border-blue-300 rounded-md hover:bg-blue-50 dark:hover:bg-blue-900/20"
          >
            Change
          </button>
          <button
            onClick={handleClear}
            disabled={saving}
            className="px-3 py-2 text-sm text-red-600 hover:text-red-700 border border-red-300 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20 disabled:opacity-50"
          >
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : 'Remove'}
          </button>
        </div>
        <p className="mt-1 text-sm text-green-600 dark:text-green-400">
          API key configured and stored securely
        </p>
      </div>
    );
  }

  return (
    <div>
      <label className="block text-sm font-medium mb-2">Groq API Key</label>
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <input
            type={showKey ? 'text' : 'password'}
            value={apiKey}
            onChange={(e) => {
              setApiKey(e.target.value);
              setError(null);
            }}
            placeholder="gsk_..."
            className="w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
          >
            {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
          </button>
        </div>
        <button
          onClick={handleSave}
          disabled={saving || !apiKey.trim()}
          className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
        >
          {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Check className="w-4 h-4" />}
          Save
        </button>
        {isEditing && (
          <button
            onClick={handleCancel}
            className="px-3 py-2 text-sm text-gray-600 hover:text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700"
          >
            <X className="w-4 h-4" />
          </button>
        )}
      </div>
      {error && (
        <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error}</p>
      )}
      <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
        Get your API key from{' '}
        <a
          href="https://console.groq.com"
          target="_blank"
          rel="noopener noreferrer"
          className="text-blue-600 hover:underline"
        >
          console.groq.com
        </a>
        . Keys are stored securely in your system's credential manager.
      </p>
    </div>
  );
}

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export function SettingsPanel() {
  const { settings, updateSettings, saving, error } = useSettings();
  const [activeTab, setActiveTab] = useState<'general' | 'transcription' | 'audio'>('general');
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);

  // Fetch audio devices on mount
  useEffect(() => {
    const loadDevices = async () => {
      try {
        const devices = await invoke<AudioDevice[]>('get_audio_devices');
        setAudioDevices(devices);
      } catch (e) {
        console.error('Failed to load audio devices:', e);
      }
    };
    loadDevices();
  }, []);

  if (!settings) {
    return <div>Loading settings...</div>;
  }

  const tabs = [
    { id: 'general' as const, label: 'General' },
    { id: 'transcription' as const, label: 'Transcription' },
    { id: 'audio' as const, label: 'Audio' },
  ];

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow">
      {/* Tabs */}
      <div className="border-b border-gray-200 dark:border-gray-700">
        <nav className="flex">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-4 py-3 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.id
                  ? 'border-blue-600 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Content */}
      <div className="p-6 space-y-6">
        {/* Error display */}
        {error && (
          <div className="p-3 text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 rounded-lg" role="alert">
            {error}
          </div>
        )}

        {activeTab === 'general' && (
          <>
            {/* Recording Mode - Toggle Switch */}
            <div>
              <label className="block text-sm font-medium mb-3">Recording Mode</label>
              <div className="flex items-center gap-3">
                <span className={`text-sm ${settings.recording.mode === 'push-to-talk' ? 'font-medium' : 'text-gray-500'}`}>
                  Push-to-Talk
                </span>
                <button
                  type="button"
                  role="switch"
                  aria-checked={settings.recording.mode === 'toggle'}
                  aria-label="Toggle between push-to-talk and toggle mode"
                  onClick={() =>
                    updateSettings({
                      ...settings,
                      recording: {
                        ...settings.recording,
                        mode: settings.recording.mode === 'push-to-talk' ? 'toggle' : 'push-to-talk',
                      },
                    })
                  }
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    settings.recording.mode === 'toggle' ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600'
                  }`}
                >
                  <span
                    aria-hidden="true"
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      settings.recording.mode === 'toggle' ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
                <span className={`text-sm ${settings.recording.mode === 'toggle' ? 'font-medium' : 'text-gray-500'}`}>
                  Toggle
                </span>
              </div>
              <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                {settings.recording.mode === 'push-to-talk'
                  ? 'Hold the shortcut key to record, release to transcribe.'
                  : 'Press once to start, press again to stop and transcribe.'}
              </p>
            </div>

            {/* Shortcut */}
            <div>
              <label className="block text-sm font-medium mb-2">Record Shortcut</label>
              <HotkeyInput
                value={settings.shortcuts.record}
                onChange={(shortcut) =>
                  updateSettings({
                    ...settings,
                    shortcuts: { ...settings.shortcuts, record: shortcut },
                  })
                }
              />
            </div>

            {/* UI Options with descriptions */}
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="start-minimized"
                  checked={settings.ui.start_minimized}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      ui: { ...settings.ui, start_minimized: e.target.checked },
                    })
                  }
                  className="rounded text-blue-600 mt-0.5"
                />
                <div>
                  <label htmlFor="start-minimized" className="font-medium text-sm cursor-pointer">
                    Start minimized to tray
                  </label>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    Launch the app hidden in the system tray instead of showing the window
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="show-indicator"
                  checked={settings.ui.show_indicator}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      ui: { ...settings.ui, show_indicator: e.target.checked },
                    })
                  }
                  className="rounded text-blue-600 mt-0.5"
                />
                <div>
                  <label htmlFor="show-indicator" className="font-medium text-sm cursor-pointer">
                    Show recording indicator
                  </label>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    Display a floating overlay showing recording status and duration
                  </p>
                </div>
              </div>
            </div>

            {/* Theme Selection */}
            <div>
              <label className="block text-sm font-medium mb-3">Appearance</label>
              <div className="grid grid-cols-3 gap-3">
                {[
                  { value: 'light' as const, icon: Sun, label: 'Light' },
                  { value: 'dark' as const, icon: Moon, label: 'Dark' },
                  { value: 'system' as const, icon: Monitor, label: 'System' },
                ].map(({ value, icon: Icon, label }) => (
                  <button
                    key={value}
                    onClick={() => {
                      updateSettings({
                        ...settings,
                        ui: { ...settings.ui, theme: value },
                      });
                      // Apply theme immediately
                      const root = document.documentElement;
                      if (value === 'dark') {
                        root.classList.add('dark');
                      } else if (value === 'light') {
                        root.classList.remove('dark');
                      } else {
                        // System preference
                        if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
                          root.classList.add('dark');
                        } else {
                          root.classList.remove('dark');
                        }
                      }
                    }}
                    className={`flex flex-col items-center gap-2 p-3 rounded-lg border-2 transition-all ${
                      settings.ui.theme === value
                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                    }`}
                  >
                    <Icon className={`w-5 h-5 ${
                      settings.ui.theme === value
                        ? 'text-blue-600 dark:text-blue-400'
                        : 'text-gray-500 dark:text-gray-400'
                    }`} />
                    <span className={`text-sm ${
                      settings.ui.theme === value
                        ? 'font-medium text-blue-600 dark:text-blue-400'
                        : 'text-gray-600 dark:text-gray-400'
                    }`}>
                      {label}
                    </span>
                  </button>
                ))}
              </div>
              <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                Choose how GigaWhisper appears. System will follow your OS settings.
              </p>
            </div>
          </>
        )}

        {activeTab === 'transcription' && (
          <>
            {/* Language - moved to top as most frequently changed */}
            <div>
              <label className="block text-sm font-medium mb-2">Language</label>
              <select
                value={settings.transcription.language}
                onChange={(e) =>
                  updateSettings({
                    ...settings,
                    transcription: { ...settings.transcription, language: e.target.value },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="auto">Auto-detect</option>
                <option value="en">English</option>
                <option value="fr">French</option>
                <option value="de">German</option>
                <option value="es">Spanish</option>
                <option value="it">Italian</option>
                <option value="pt">Portuguese</option>
                <option value="ja">Japanese</option>
                <option value="ko">Korean</option>
                <option value="zh">Chinese</option>
              </select>
              <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                Select the language you'll be speaking for better accuracy
              </p>
            </div>

            {/* Provider Selection */}
            <div>
              <label className="block text-sm font-medium mb-2">Provider</label>
              <ProviderToggle
                value={settings.transcription.provider}
                onChange={(provider) =>
                  updateSettings({
                    ...settings,
                    transcription: { ...settings.transcription, provider },
                  })
                }
              />
            </div>

            {/* Local Settings */}
            {settings.transcription.provider === 'local' && (
              <>
                <div>
                  <label className="block text-sm font-medium mb-2">Whisper Model</label>
                  <ModelSelector
                    value={settings.transcription.local.model}
                    onChange={(model) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          local: { ...settings.transcription.local, model },
                        },
                      })
                    }
                  />
                </div>

                {/* GPU Acceleration Toggle */}
                <div className="flex items-start gap-3">
                  <input
                    type="checkbox"
                    id="gpu-enabled"
                    checked={settings.transcription.local.gpu_enabled}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          local: { ...settings.transcription.local, gpu_enabled: e.target.checked },
                        },
                      })
                    }
                    className="rounded text-blue-600 mt-0.5"
                  />
                  <div>
                    <label htmlFor="gpu-enabled" className="font-medium text-sm cursor-pointer">
                      GPU Acceleration
                    </label>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      Use your graphics card for faster transcription (requires compatible GPU)
                    </p>
                  </div>
                </div>
              </>
            )}

            {/* Groq Settings */}
            {settings.transcription.provider === 'groq' && (
              <GroqApiKeyInput />
            )}
          </>
        )}

        {activeTab === 'audio' && (
          <>
            {/* Input Device */}
            <div>
              <label className="block text-sm font-medium mb-2">Input Device</label>
              <select
                value={settings.audio.input_device || ''}
                onChange={(e) =>
                  updateSettings({
                    ...settings,
                    audio: {
                      ...settings.audio,
                      input_device: e.target.value || null,
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="">Default Microphone</option>
                {audioDevices.map((device) => (
                  <option key={device.name} value={device.name}>
                    {device.name}{device.is_default ? ' (System Default)' : ''}
                  </option>
                ))}
              </select>
              {audioDevices.length === 0 && (
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  No additional microphones detected
                </p>
              )}
            </div>

            {/* Output Options with descriptions */}
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="auto-capitalize"
                  checked={settings.output.auto_capitalize}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      output: { ...settings.output, auto_capitalize: e.target.checked },
                    })
                  }
                  className="rounded text-blue-600 mt-0.5"
                />
                <div>
                  <label htmlFor="auto-capitalize" className="font-medium text-sm cursor-pointer">
                    Auto-capitalize first letter
                  </label>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    Automatically capitalize the first letter of the transcribed text
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="auto-punctuation"
                  checked={settings.output.auto_punctuation}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      output: { ...settings.output, auto_punctuation: e.target.checked },
                    })
                  }
                  className="rounded text-blue-600 mt-0.5"
                />
                <div>
                  <label htmlFor="auto-punctuation" className="font-medium text-sm cursor-pointer">
                    Auto-punctuation
                  </label>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    Add a period at the end if no punctuation is detected
                  </p>
                </div>
              </div>
            </div>
          </>
        )}

        {/* Save Indicator */}
        {saving && (
          <div className="text-sm text-gray-500 dark:text-gray-400">
            Saving...
          </div>
        )}
      </div>
    </div>
  );
}
