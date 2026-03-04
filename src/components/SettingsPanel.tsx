import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSettings } from '../hooks/useSettings';
import { usePremium } from '../hooks/usePremium';
import { HotkeyInput } from './HotkeyInput';
import { ModelSelector } from './ModelSelector';
import { LicensePanel } from './premium/LicensePanel';
import { ContextManager } from './premium/ContextManager';
import { PostProcessingConfig } from './premium/PostProcessingConfig';
import { PremiumBadge } from './premium/PremiumBadge';
import { Sun, Moon, Monitor } from 'lucide-react';

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export function SettingsPanel() {
  const { settings, updateSettings, saving, error } = useSettings();
  const { status: premiumStatus } = usePremium();
  const isPremium = premiumStatus?.is_premium ?? false;
  const [activeTab, setActiveTab] = useState<'general' | 'transcription' | 'audio' | 'premium' | 'contexts' | 'post-processing'>('general');
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
    return (
      <div className="flex items-center justify-center py-12">
        <div className="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  const tabs = [
    { id: 'general' as const, label: 'General' },
    { id: 'transcription' as const, label: 'Transcription' },
    { id: 'audio' as const, label: 'Audio' },
    { id: 'contexts' as const, label: 'Contexts', premium: true },
    { id: 'post-processing' as const, label: 'Post-Processing', premium: true },
    { id: 'premium' as const, label: 'Premium' },
  ];

  return (
    <div className="card animate-fade-in max-w-2xl mx-auto">
      {/* Tabs */}
      <div className="border-b border-gray-200/80 dark:border-violet-500/10">
        <nav className="flex px-2">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`relative px-4 py-3 text-sm font-medium transition-all duration-150 -mb-px flex items-center gap-1.5 ${
                activeTab === tab.id
                  ? 'text-indigo-600 dark:text-indigo-400'
                  : 'border-b-2 border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              {tab.label}
              {'premium' in tab && tab.premium && <PremiumBadge locked={!isPremium} />}
              {activeTab === tab.id && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-indigo-500 to-violet-500 rounded-full" />
              )}
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
                    settings.recording.mode === 'toggle' ? 'bg-indigo-500' : 'bg-gray-300 dark:bg-[#252136]'
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
                  className="rounded text-indigo-600 mt-0.5"
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
                  className="rounded text-indigo-600 mt-0.5"
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
                        ? 'border-indigo-500 bg-indigo-50 dark:bg-indigo-900/20'
                        : 'border-gray-200 dark:border-violet-500/15 hover:border-gray-300 dark:hover:border-gray-600'
                    }`}
                  >
                    <Icon className={`w-5 h-5 ${
                      settings.ui.theme === value
                        ? 'text-indigo-600 dark:text-indigo-400'
                        : 'text-gray-500 dark:text-gray-400'
                    }`} />
                    <span className={`text-sm ${
                      settings.ui.theme === value
                        ? 'font-medium text-indigo-600 dark:text-indigo-400'
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-violet-500/20 rounded-md bg-white dark:bg-[#252136] focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
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

            {/* Whisper Model */}
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
                className="rounded text-indigo-600 mt-0.5"
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-violet-500/20 rounded-md bg-white dark:bg-[#252136] focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
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
                  className="rounded text-indigo-600 mt-0.5"
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
                  className="rounded text-indigo-600 mt-0.5"
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

        {activeTab === 'contexts' && (
          <ContextManager />
        )}

        {activeTab === 'post-processing' && (
          <PostProcessingConfig />
        )}

        {activeTab === 'premium' && (
          <LicensePanel />
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
