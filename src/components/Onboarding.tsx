import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Mic, Settings, Sparkles, ChevronRight, ChevronLeft, Keyboard, Sun, Moon, Monitor, Star, Download, Check, Loader2 } from 'lucide-react';

interface OnboardingProps {
  onComplete: () => void;
}

type Theme = 'light' | 'dark' | 'system';
type WhisperModel = 'tiny' | 'base' | 'small' | 'medium' | 'large';

interface ModelOption {
  model: WhisperModel;
  name: string;
  size: string;
  description: string;
  recommended?: boolean;
}

const MODELS: ModelOption[] = [
  { model: 'tiny', name: 'Tiny', size: '75 MB', description: 'Fastest, basic accuracy' },
  { model: 'base', name: 'Base', size: '142 MB', description: 'Fast, good accuracy' },
  { model: 'small', name: 'Small', size: '466 MB', description: 'Balanced speed & accuracy', recommended: true },
  { model: 'medium', name: 'Medium', size: '1.5 GB', description: 'Slower, high accuracy' },
];

export function Onboarding({ onComplete }: OnboardingProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedTheme, setSelectedTheme] = useState<Theme>('system');
  const [selectedModel, setSelectedModel] = useState<WhisperModel>('small');
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadComplete, setDownloadComplete] = useState(false);

  // Listen for download progress
  useEffect(() => {
    const unlistenProgress = listen<{ model: string; percentage: number }>('model-download-progress', (event) => {
      setDownloadProgress(event.payload.percentage);
    });

    const unlistenComplete = listen<{ model: string }>('model-download-complete', () => {
      setDownloading(false);
      setDownloadComplete(true);
    });

    const unlistenError = listen<{ model: string; error: string }>('model-download-error', () => {
      setDownloading(false);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Apply theme immediately when changed
  useEffect(() => {
    const root = document.documentElement;
    if (selectedTheme === 'dark') {
      root.classList.add('dark');
    } else if (selectedTheme === 'light') {
      root.classList.remove('dark');
    } else {
      // System preference
      if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
        root.classList.add('dark');
      } else {
        root.classList.remove('dark');
      }
    }
  }, [selectedTheme]);

  const handleStartDownload = async () => {
    setDownloading(true);
    setDownloadProgress(0);
    try {
      await invoke('download_model', { model: selectedModel });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading(false);
    }
  };

  const saveSettings = async () => {
    try {
      // Get current settings
      const settings = await invoke<Record<string, unknown>>('get_settings');

      // Update theme and model
      const updatedSettings = {
        ...settings,
        ui: {
          ...(settings.ui as Record<string, unknown>),
          theme: selectedTheme,
        },
        transcription: {
          ...(settings.transcription as Record<string, unknown>),
          local: {
            ...((settings.transcription as Record<string, unknown>).local as Record<string, unknown>),
            model: selectedModel,
          },
        },
      };

      await invoke('save_settings', { settings: updatedSettings });
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  };

  const handleComplete = async () => {
    await saveSettings();
    onComplete();
  };

  const handleNext = () => {
    if (currentStep < 3) {
      setCurrentStep(currentStep + 1);
    } else {
      handleComplete();
    }
  };

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  // Step 0: Theme Selection
  const renderThemeStep = () => (
    <>
      <div className="p-6 pb-0">
        <div className="flex items-center gap-3 mb-2">
          <div className="p-2 bg-purple-100 dark:bg-purple-900/30 rounded-xl text-purple-600 dark:text-purple-400">
            <Sun className="w-6 h-6" />
          </div>
          <h2 className="text-xl font-bold text-gray-900 dark:text-white">
            Choose Your Theme
          </h2>
        </div>
        <p className="text-gray-600 dark:text-gray-400 text-sm leading-relaxed">
          Select how you want GigaWhisper to look. You can change this anytime in settings.
        </p>
      </div>

      <div className="p-8 flex items-center justify-center">
        <div className="grid grid-cols-3 gap-4 w-full max-w-sm">
          {[
            { value: 'light' as Theme, icon: Sun, label: 'Light', bg: 'bg-white', border: 'border-gray-200' },
            { value: 'dark' as Theme, icon: Moon, label: 'Dark', bg: 'bg-gray-800', border: 'border-gray-700' },
            { value: 'system' as Theme, icon: Monitor, label: 'System', bg: 'bg-gradient-to-br from-white to-gray-800', border: 'border-gray-400' },
          ].map(({ value, icon: Icon, label, bg, border }) => (
            <button
              key={value}
              onClick={() => setSelectedTheme(value)}
              className={`flex flex-col items-center gap-3 p-4 rounded-xl border-2 transition-all ${
                selectedTheme === value
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                  : `border-gray-200 dark:border-gray-700 hover:border-gray-300`
              }`}
            >
              <div className={`w-16 h-12 rounded-lg ${bg} ${border} border flex items-center justify-center`}>
                <Icon className={`w-5 h-5 ${value === 'dark' ? 'text-white' : value === 'system' ? 'text-gray-600' : 'text-gray-700'}`} />
              </div>
              <span className="text-sm font-medium">{label}</span>
              {selectedTheme === value && (
                <div className="w-2 h-2 bg-blue-500 rounded-full" />
              )}
            </button>
          ))}
        </div>
      </div>
    </>
  );

  // Step 1: How it works
  const renderHowItWorksStep = () => (
    <>
      <div className="p-6 pb-0">
        <div className="flex items-center gap-3 mb-2">
          <div className="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-xl text-blue-600 dark:text-blue-400">
            <Mic className="w-6 h-6" />
          </div>
          <h2 className="text-xl font-bold text-gray-900 dark:text-white">
            Voice to Text, Instantly
          </h2>
        </div>
        <p className="text-gray-600 dark:text-gray-400 text-sm leading-relaxed">
          Press your shortcut key, speak, and your words are automatically typed wherever your cursor is.
        </p>
      </div>

      <div className="p-8 flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          {/* Keyboard shortcut */}
          <div className="flex items-center gap-2">
            <div className="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg border border-gray-300 dark:border-gray-600 font-mono text-sm">
              Ctrl
            </div>
            <span className="text-gray-400">+</span>
            <div className="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg border border-gray-300 dark:border-gray-600 font-mono text-sm">
              Shift
            </div>
            <span className="text-gray-400">+</span>
            <div className="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg border border-gray-300 dark:border-gray-600 font-mono text-sm">
              Space
            </div>
          </div>

          {/* Arrow */}
          <div className="text-blue-500">
            <svg className="w-6 h-6 animate-bounce" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
          </div>

          {/* Recording indicator */}
          <div className="flex items-center gap-3 px-4 py-3 bg-red-50 dark:bg-red-900/20 rounded-xl">
            <div className="relative">
              <div className="w-3 h-3 bg-red-500 rounded-full" />
              <div className="absolute inset-0 w-3 h-3 bg-red-500 rounded-full animate-ping" />
            </div>
            <span className="text-red-600 dark:text-red-400 font-medium">Recording...</span>
            <div className="flex gap-0.5">
              {[...Array(5)].map((_, i) => (
                <div
                  key={i}
                  className="w-1 bg-red-400 rounded-full animate-pulse"
                  style={{
                    height: `${12 + Math.random() * 12}px`,
                    animationDelay: `${i * 0.1}s`,
                  }}
                />
              ))}
            </div>
          </div>

          {/* Arrow */}
          <div className="text-green-500">
            <svg className="w-6 h-6 animate-bounce" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
          </div>

          {/* Text output */}
          <div className="px-4 py-3 bg-green-50 dark:bg-green-900/20 rounded-xl border-2 border-dashed border-green-300 dark:border-green-700">
            <span className="text-green-700 dark:text-green-300">Hello, this is my transcribed text!</span>
            <span className="inline-block w-0.5 h-4 bg-green-500 ml-1 animate-pulse" />
          </div>
        </div>
      </div>
    </>
  );

  // Step 2: Model Selection (interactive)
  const renderModelStep = () => (
    <>
      <div className="p-6 pb-0">
        <div className="flex items-center gap-3 mb-2">
          <div className="p-2 bg-orange-100 dark:bg-orange-900/30 rounded-xl text-orange-600 dark:text-orange-400">
            <Settings className="w-6 h-6" />
          </div>
          <h2 className="text-xl font-bold text-gray-900 dark:text-white">
            Choose Your Model
          </h2>
        </div>
        <p className="text-gray-600 dark:text-gray-400 text-sm leading-relaxed">
          Select a transcription model. Larger models are more accurate but slower. You can change this later in settings.
        </p>
      </div>

      <div className="p-6 flex flex-col items-center">
        <div className="space-y-2 w-full max-w-sm">
          {MODELS.map((model) => (
            <button
              key={model.model}
              onClick={() => {
                setSelectedModel(model.model);
                setDownloadComplete(false);
              }}
              disabled={downloading}
              className={`w-full flex items-center justify-between p-3 rounded-lg border-2 transition-all text-left ${
                selectedModel === model.model
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              } ${downloading ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              <div className="flex items-center gap-3">
                <div
                  className={`w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
                    selectedModel === model.model
                      ? 'border-blue-500 bg-blue-500'
                      : 'border-gray-300 dark:border-gray-600'
                  }`}
                >
                  {selectedModel === model.model && (
                    <Check className="w-2.5 h-2.5 text-white" />
                  )}
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm">{model.name}</span>
                    {model.recommended && (
                      <span className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
                        <Star className="w-3 h-3 fill-current" />
                        Recommended
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {model.size} - {model.description}
                  </div>
                </div>
              </div>
            </button>
          ))}
        </div>

        {/* Download button */}
        <div className="mt-6 w-full max-w-sm">
          {downloading ? (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="flex items-center gap-2 text-blue-600 dark:text-blue-400">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Downloading {MODELS.find(m => m.model === selectedModel)?.name}...
                </span>
                <span className="text-gray-500">{downloadProgress.toFixed(0)}%</span>
              </div>
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  className="bg-blue-500 h-2 rounded-full transition-all"
                  style={{ width: `${downloadProgress}%` }}
                />
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400 text-center">
                Download continues in background. You can proceed to next step.
              </p>
            </div>
          ) : downloadComplete ? (
            <div className="flex items-center justify-center gap-2 text-green-600 dark:text-green-400">
              <Check className="w-5 h-5" />
              <span className="font-medium">Model ready!</span>
            </div>
          ) : (
            <button
              onClick={handleStartDownload}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
            >
              <Download className="w-4 h-4" />
              Download {MODELS.find(m => m.model === selectedModel)?.name} Model
            </button>
          )}
        </div>
      </div>
    </>
  );

  // Step 3: Ready to go
  const renderReadyStep = () => (
    <>
      <div className="p-6 pb-0">
        <div className="flex items-center gap-3 mb-2">
          <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-xl text-green-600 dark:text-green-400">
            <Sparkles className="w-6 h-6" />
          </div>
          <h2 className="text-xl font-bold text-gray-900 dark:text-white">
            Ready to Go!
          </h2>
        </div>
        <p className="text-gray-600 dark:text-gray-400 text-sm leading-relaxed">
          GigaWhisper runs in your system tray. Use your shortcut anytime, anywhere.
        </p>
      </div>

      <div className="p-8 flex items-center justify-center">
        <div className="flex flex-col items-center gap-6">
          {/* System tray illustration */}
          <div className="flex items-center gap-2 px-4 py-2 bg-gray-800 rounded-lg">
            <div className="flex gap-1">
              <div className="w-4 h-4 bg-gray-600 rounded" />
              <div className="w-4 h-4 bg-gray-600 rounded" />
              <div className="w-4 h-4 bg-blue-500 rounded flex items-center justify-center">
                <Mic className="w-2.5 h-2.5 text-white" />
              </div>
              <div className="w-4 h-4 bg-gray-600 rounded" />
            </div>
            <div className="text-xs text-gray-400 ml-2">11:42</div>
          </div>

          {/* Tips */}
          <div className="space-y-3 text-sm">
            <div className="flex items-start gap-3">
              <div className="p-1.5 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                <Keyboard className="w-4 h-4 text-blue-600 dark:text-blue-400" />
              </div>
              <div>
                <div className="font-medium">Customize your shortcut</div>
                <div className="text-gray-500 dark:text-gray-400 text-xs">Settings → General → Record Shortcut</div>
              </div>
            </div>
            <div className="flex items-start gap-3">
              <div className="p-1.5 bg-purple-100 dark:bg-purple-900/30 rounded-lg">
                <Settings className="w-4 h-4 text-purple-600 dark:text-purple-400" />
              </div>
              <div>
                <div className="font-medium">Switch modes</div>
                <div className="text-gray-500 dark:text-gray-400 text-xs">Push-to-talk or Toggle mode</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );

  const renderStep = () => {
    switch (currentStep) {
      case 0:
        return renderThemeStep();
      case 1:
        return renderHowItWorksStep();
      case 2:
        return renderModelStep();
      case 3:
        return renderReadyStep();
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-gray-900/50 backdrop-blur-sm">
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden">
        {renderStep()}

        {/* Footer */}
        <div className="p-6 pt-0 flex items-center justify-between">
          {/* Step indicators */}
          <div className="flex gap-2">
            {[0, 1, 2, 3].map((index) => (
              <button
                key={index}
                onClick={() => setCurrentStep(index)}
                className={`h-2 rounded-full transition-all ${
                  index === currentStep
                    ? 'w-6 bg-blue-600'
                    : 'w-2 bg-gray-300 dark:bg-gray-600 hover:bg-gray-400'
                }`}
              />
            ))}
          </div>

          {/* Navigation buttons */}
          <div className="flex gap-2">
            {currentStep > 0 && (
              <button
                onClick={handlePrev}
                className="flex items-center gap-1 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
              >
                <ChevronLeft className="w-4 h-4" />
                Back
              </button>
            )}
            <button
              onClick={handleNext}
              className="flex items-center gap-1 px-5 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
            >
              {currentStep === 3 ? (
                'Get Started'
              ) : (
                <>
                  Next
                  <ChevronRight className="w-4 h-4" />
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
