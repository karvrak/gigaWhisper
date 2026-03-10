import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from '../hooks/useSettings';
import { HotkeyInput } from './HotkeyInput';
import { ModelSelector } from './ModelSelector';
import { LicensePanel } from './premium/LicensePanel';
import { ThemeSelector, applyCustomTheme } from './premium/ThemeSelector';
import { THEME_PRESETS } from '../themes/presets';
import { ContextManager } from './premium/ContextManager';
import { PostProcessingConfig } from './premium/PostProcessingConfig';
import { ApiKeyInput } from './premium/ApiKeyInput';
import { CreditsMeter } from './premium/CreditsMeter';
import { usePremium } from '../hooks/usePremium';
import { Sun, Moon, Monitor } from 'lucide-react';

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export function SettingsPanel() {
  const { settings, updateSettings, resetSettings, saving, error } = useSettings();
  const { isPremium } = usePremium();
  const [activeTab, setActiveTab] = useState<'general' | 'transcription' | 'audio' | 'premium'>('general');
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

  // Apply saved custom theme on load
  useEffect(() => {
    if (!settings?.ui.custom_theme) return;
    const theme = THEME_PRESETS.find((t) => t.id === settings.ui.custom_theme);
    applyCustomTheme(theme ?? null);
  }, [settings?.ui.custom_theme]);

  if (!settings) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="w-6 h-6 border-2 border-accent border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  const tabs = [
    { id: 'general' as const, label: 'General' },
    { id: 'transcription' as const, label: 'Transcription' },
    { id: 'audio' as const, label: 'Audio' },
    { id: 'premium' as const, label: 'Premium' },
  ];

  return (
    <div className="card animate-fade-in max-w-2xl mx-auto">
      {/* Tab selector */}
      <div className="border-b border-edge px-6 py-3">
        <select
          data-testid="tab-selector"
          value={activeTab}
          onChange={(e) => setActiveTab(e.target.value as typeof activeTab)}
          className="w-full px-3 py-2 text-sm font-medium border border-edge rounded-md bg-surface-tertiary focus:ring-2 focus:ring-accent focus:border-accent"
        >
          {tabs.map((tab) => (
            <option key={tab.id} value={tab.id}>
              {tab.label}
            </option>
          ))}
        </select>
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
                <span className={`text-sm ${settings.recording.mode === 'push-to-talk' ? 'font-medium' : 'text-content-secondary'}`}>
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
                    settings.recording.mode === 'toggle' ? 'bg-accent' : 'bg-surface-tertiary'
                  }`}
                >
                  <span
                    aria-hidden="true"
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      settings.recording.mode === 'toggle' ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
                <span className={`text-sm ${settings.recording.mode === 'toggle' ? 'font-medium' : 'text-content-secondary'}`}>
                  Toggle
                </span>
              </div>
              <p className="mt-2 text-sm text-content-secondary">
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
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="start-minimized" className="font-medium text-sm cursor-pointer">
                    Start minimized to tray
                  </label>
                  <p className="text-xs text-content-secondary">
                    Launch the app hidden in the system tray instead of showing the window
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="auto-start"
                  checked={settings.ui.auto_start}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      ui: { ...settings.ui, auto_start: e.target.checked },
                    })
                  }
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="auto-start" className="font-medium text-sm cursor-pointer">
                    Start with Windows
                  </label>
                  <p className="text-xs text-content-secondary">
                    Automatically launch GigaWhisper when you log in to Windows
                  </p>
                </div>
              </div>

              <div className="flex items-start gap-3">
                <input
                  type="checkbox"
                  id="auto-update"
                  checked={settings.ui.auto_update}
                  onChange={(e) =>
                    updateSettings({
                      ...settings,
                      ui: { ...settings.ui, auto_update: e.target.checked },
                    })
                  }
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="auto-update" className="font-medium text-sm cursor-pointer">
                    Auto-update
                  </label>
                  <p className="text-xs text-content-secondary">
                    Automatically download and install updates without prompting
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
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="show-indicator" className="font-medium text-sm cursor-pointer">
                    Show recording indicator
                  </label>
                  <p className="text-xs text-content-secondary">
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
                      // Apply theme immediately (only if no custom theme active)
                      const root = document.documentElement;
                      if (!root.hasAttribute('data-custom-theme')) {
                        if (value === 'dark') {
                          root.classList.add('dark');
                        } else if (value === 'light') {
                          root.classList.remove('dark');
                        } else {
                          if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
                            root.classList.add('dark');
                          } else {
                            root.classList.remove('dark');
                          }
                        }
                      }
                    }}
                    className={`flex flex-col items-center gap-2 p-3 rounded-lg border-2 transition-all ${
                      settings.ui.theme === value
                        ? 'border-accent bg-accent-subtle'
                        : 'border-edge hover:border-edge-hover'
                    }`}
                  >
                    <Icon className={`w-5 h-5 ${
                      settings.ui.theme === value
                        ? 'text-accent'
                        : 'text-content-secondary'
                    }`} />
                    <span className={`text-sm ${
                      settings.ui.theme === value
                        ? 'font-medium text-accent'
                        : 'text-content-secondary'
                    }`}>
                      {label}
                    </span>
                  </button>
                ))}
              </div>
              <p className="mt-2 text-xs text-content-secondary">
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
                className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary focus:ring-2 focus:ring-accent focus:border-accent"
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
              <p className="mt-1 text-xs text-content-secondary">
                Select the language you'll be speaking for better accuracy
              </p>
            </div>

            {/* Transcription Provider */}
            <div>
              <label className="block text-sm font-medium mb-2">Transcription Provider</label>
              <select
                value={settings.transcription.provider}
                onChange={(e) =>
                  updateSettings({
                    ...settings,
                    transcription: {
                      ...settings.transcription,
                      provider: e.target.value as 'local' | 'groq' | 'openai' | 'deepgram' | 'custom',
                    },
                  })
                }
                className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary focus:ring-2 focus:ring-accent focus:border-accent"
              >
                <option value="local">Local (Whisper)</option>
                <option value="groq">
                  Groq Cloud{!settings.transcription.groq.api_key_configured ? ' (No API key)' : ''}
                </option>
                <option value="openai">
                  OpenAI Whisper{!settings.transcription.openai.api_key_configured ? ' (No API key)' : ''}
                </option>
                <option value="deepgram">
                  Deepgram Nova{!settings.transcription.deepgram.api_key_configured ? ' (No API key)' : ''}
                </option>
                <option value="custom">
                  Custom Endpoint{!settings.transcription.custom.api_key_configured ? ' (No API key)' : ''}
                </option>
              </select>
              {(settings.transcription.provider !== 'local' && !({
                groq: settings.transcription.groq.api_key_configured,
                openai: settings.transcription.openai.api_key_configured,
                deepgram: settings.transcription.deepgram.api_key_configured,
                custom: settings.transcription.custom.api_key_configured,
              } as Record<string, boolean>)[settings.transcription.provider]) && (
                <p className="mt-1 text-xs text-red-500">
                  API key not configured. Please add your API key below before using this provider.
                </p>
              )}
            </div>

            {/* Provider API Key */}
            {settings.transcription.provider === 'groq' && (
              <ApiKeyInput
                provider="Groq"
                label="Groq API Key"
                placeholder="gsk_..."
                isConfigured={settings.transcription.groq.api_key_configured}
                setCommand="set_groq_api_key"
                clearCommand="clear_groq_api_key"
                validateCommand="validate_groq_api_key_live"
                onStatusChange={resetSettings}
              />
            )}
            {settings.transcription.provider === 'openai' && (
              <ApiKeyInput
                provider="OpenAI"
                label="OpenAI API Key"
                placeholder="sk-..."
                isConfigured={settings.transcription.openai.api_key_configured}
                setCommand="set_openai_api_key"
                clearCommand="clear_openai_api_key"
                validateCommand="validate_openai_api_key_live"
                onStatusChange={resetSettings}
              />
            )}
            {settings.transcription.provider === 'deepgram' && (
              <ApiKeyInput
                provider="Deepgram"
                label="Deepgram API Key"
                placeholder="Your Deepgram API key"
                isConfigured={settings.transcription.deepgram.api_key_configured}
                setCommand="set_deepgram_api_key"
                clearCommand="clear_deepgram_api_key"
                validateCommand="validate_deepgram_api_key_live"
                onStatusChange={resetSettings}
              />
            )}
            {settings.transcription.provider === 'custom' && (
              <>
                <div>
                  <label className="block text-xs font-medium mb-1">API URL</label>
                  <input
                    type="text"
                    value={settings.transcription.custom.api_url}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          custom: { ...settings.transcription.custom, api_url: e.target.value },
                        },
                      })
                    }
                    placeholder="http://localhost:8080/v1/audio/transcriptions"
                    className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium mb-1">Model</label>
                  <input
                    type="text"
                    value={settings.transcription.custom.model}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          custom: { ...settings.transcription.custom, model: e.target.value },
                        },
                      })
                    }
                    placeholder="whisper-large-v3"
                    className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium mb-1">Authentication Type</label>
                  <select
                    value={settings.transcription.custom.auth_type}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          custom: {
                            ...settings.transcription.custom,
                            auth_type: e.target.value as 'bearer' | 'x-api-key' | 'custom',
                          },
                        },
                      })
                    }
                    className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                  >
                    <option value="bearer">Bearer Token</option>
                    <option value="x-api-key">x-api-key Header</option>
                    <option value="custom">Custom Header</option>
                  </select>
                </div>
                {settings.transcription.custom.auth_type === 'custom' && (
                  <div>
                    <label className="block text-xs font-medium mb-1">Custom Header Name</label>
                    <input
                      type="text"
                      value={settings.transcription.custom.custom_header_name}
                      onChange={(e) =>
                        updateSettings({
                          ...settings,
                          transcription: {
                            ...settings.transcription,
                            custom: { ...settings.transcription.custom, custom_header_name: e.target.value },
                          },
                        })
                      }
                      placeholder="X-My-Auth"
                      className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                    />
                  </div>
                )}
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="custom-transcription-invalid-certs"
                    checked={settings.transcription.custom.accept_invalid_certs}
                    onChange={(e) =>
                      updateSettings({
                        ...settings,
                        transcription: {
                          ...settings.transcription,
                          custom: { ...settings.transcription.custom, accept_invalid_certs: e.target.checked },
                        },
                      })
                    }
                    className="rounded"
                  />
                  <label htmlFor="custom-transcription-invalid-certs" className="text-xs cursor-pointer">
                    Accept self-signed certificates
                  </label>
                </div>
                <ApiKeyInput
                  provider="Custom"
                  label="API Key"
                  placeholder="Your API key"
                  isConfigured={settings.transcription.custom.api_key_configured}
                  setCommand="set_custom_transcription_api_key"
                  clearCommand="clear_custom_transcription_api_key"
                  validateCommand="validate_custom_transcription_api_key_live"
                  onStatusChange={resetSettings}
                />
              </>
            )}

            {/* Whisper Model - only for local provider */}
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
                    className="rounded mt-0.5"
                  />
                  <div>
                    <label htmlFor="gpu-enabled" className="font-medium text-sm cursor-pointer">
                      GPU Acceleration
                    </label>
                    <p className="text-xs text-content-secondary">
                      Use your graphics card for faster transcription (requires compatible GPU)
                    </p>
                  </div>
                </div>
              </>
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
                className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary focus:ring-2 focus:ring-accent focus:border-accent"
              >
                <option value="">Default Microphone</option>
                {audioDevices.map((device) => (
                  <option key={device.name} value={device.name}>
                    {device.name}{device.is_default ? ' (System Default)' : ''}
                  </option>
                ))}
              </select>
              {audioDevices.length === 0 && (
                <p className="mt-1 text-xs text-content-secondary">
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
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="auto-capitalize" className="font-medium text-sm cursor-pointer">
                    Auto-capitalize first letter
                  </label>
                  <p className="text-xs text-content-secondary">
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
                  className="rounded mt-0.5"
                />
                <div>
                  <label htmlFor="auto-punctuation" className="font-medium text-sm cursor-pointer">
                    Auto-punctuation
                  </label>
                  <p className="text-xs text-content-secondary">
                    Add a period at the end if no punctuation is detected
                  </p>
                </div>
              </div>
            </div>
          </>
        )}

        {activeTab === 'premium' && (
          <>
            <LicensePanel />

            {isPremium && (
              <>
                {/* Credits */}
                <CreditsMeter />

                {/* Divider */}
                <div className="border-t border-edge" />

                {/* Custom Themes */}
                <ThemeSelector
                  currentTheme={settings?.ui.custom_theme ?? null}
                  onChange={(themeId) => {
                    if (!settings) return;
                    updateSettings({
                      ...settings,
                      ui: { ...settings.ui, custom_theme: themeId },
                    });
                    // When deselecting custom theme, restore user's preferred theme
                    if (!themeId) {
                      const root = document.documentElement;
                      const theme = settings.ui.theme;
                      if (theme === 'dark') {
                        root.classList.add('dark');
                      } else if (theme === 'light') {
                        root.classList.remove('dark');
                      } else {
                        if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
                          root.classList.add('dark');
                        } else {
                          root.classList.remove('dark');
                        }
                      }
                    }
                  }}
                />

                {/* Divider */}
                <div className="border-t border-edge" />

                {/* Post-Processing */}
                <PostProcessingConfig />

                {/* Divider */}
                <div className="border-t border-edge" />

                {/* Multi-Context */}
                <ContextManager />
              </>
            )}
          </>
        )}

        {/* Save Indicator */}
        {saving && (
          <div className="text-sm text-content-secondary">
            Saving...
          </div>
        )}
      </div>
    </div>
  );
}
