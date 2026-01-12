import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SettingsPanel } from './components/SettingsPanel';
import { PopupOverlay } from './components/PopupOverlay';
import { HistoryPanel } from './components/HistoryPanel';
import { Onboarding } from './components/Onboarding';
import { useSettings } from './hooks/useSettings';
import { Minus, X } from 'lucide-react';

const ONBOARDING_KEY = 'gigawhisper_onboarding_completed';

type View = 'main' | 'history' | 'settings';

// Helper function to apply theme
function applyTheme(theme: 'system' | 'light' | 'dark') {
  const root = document.documentElement;
  if (theme === 'dark') {
    root.classList.add('dark');
  } else if (theme === 'light') {
    root.classList.remove('dark');
  } else {
    // System preference
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }
}

function App() {
  const [view, setView] = useState<View>('main');
  const { settings, loading: settingsLoading } = useSettings();
  const [showOnboarding, setShowOnboarding] = useState(() => {
    return localStorage.getItem(ONBOARDING_KEY) !== 'true';
  });

  const handleOnboardingComplete = () => {
    localStorage.setItem(ONBOARDING_KEY, 'true');
    setShowOnboarding(false);
  };

  // Apply theme when settings are loaded or changed
  useEffect(() => {
    if (settings?.ui?.theme) {
      applyTheme(settings.ui.theme);
    }
  }, [settings?.ui?.theme]);

  // Listen for system theme changes when in 'system' mode
  useEffect(() => {
    if (settings?.ui?.theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handleChange = () => applyTheme('system');
      mediaQuery.addEventListener('change', handleChange);
      return () => mediaQuery.removeEventListener('change', handleChange);
    }
  }, [settings?.ui?.theme]);

  // Listen for navigation events from tray
  useEffect(() => {
    const unsubscribe = listen('navigate:settings', () => {
      setView('settings');
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  }, []);

  if (settingsLoading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const handleMinimize = async () => {
    const win = getCurrentWindow();
    await win.minimize();
  };
  const handleClose = async () => {
    const win = getCurrentWindow();
    await win.hide();
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100 overflow-hidden">
      {/* Custom Titlebar */}
      <header
        className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 select-none flex-shrink-0"
      >
        <div className="flex items-center h-10">
          {/* Logo - left side */}
          <div className="w-10 h-10 flex items-center justify-center" data-tauri-drag-region>
            <img src="/icon.ico" alt="GigaWhisper" className="w-5 h-5" />
          </div>

          {/* Navigation - centered */}
          <nav className="flex-1 flex justify-center gap-1" data-tauri-drag-region>
            <button
              onClick={() => setView('main')}
              className={`px-3 py-1 rounded-md text-sm ${
                view === 'main'
                  ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-700'
              }`}
            >
              Home
            </button>
            <button
              onClick={() => setView('history')}
              className={`px-3 py-1 rounded-md text-sm ${
                view === 'history'
                  ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-700'
              }`}
            >
              History
            </button>
            <button
              onClick={() => setView('settings')}
              className={`px-3 py-1 rounded-md text-sm ${
                view === 'settings'
                  ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                  : 'hover:bg-gray-100 dark:hover:bg-gray-700'
              }`}
            >
              Settings
            </button>
          </nav>

          {/* Window Controls */}
          <div className="flex" style={{ WebkitAppRegion: 'no-drag' } as React.CSSProperties}>
            <button
              onClick={handleMinimize}
              onMouseDown={(e) => e.stopPropagation()}
              className="w-10 h-10 flex items-center justify-center hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              title="Minimize"
            >
              <Minus className="w-4 h-4 pointer-events-none" />
            </button>
            <button
              onClick={handleClose}
              onMouseDown={(e) => e.stopPropagation()}
              className="w-10 h-10 flex items-center justify-center hover:bg-red-500 hover:text-white transition-colors"
              title="Close"
            >
              <X className="w-4 h-4 pointer-events-none" />
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto overflow-x-hidden p-4">
        {view === 'main' && (
          <div className="space-y-6">
            {/* Shortcut Info */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
              <h2 className="text-lg font-medium mb-4">Keyboard Shortcut</h2>
              <div className="flex items-center justify-between">
                <span className="text-gray-600 dark:text-gray-400">
                  {settings?.recording.mode === 'push-to-talk'
                    ? 'Hold to record'
                    : 'Press to toggle recording'}
                </span>
                <kbd className="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md font-mono text-sm">
                  {settings?.shortcuts.record || 'Ctrl+Space'}
                </kbd>
              </div>
            </div>

            {/* Provider Info */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
              <h2 className="text-lg font-medium mb-4">Transcription Provider</h2>
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-medium capitalize">
                    {settings?.transcription.provider || 'local'}
                  </div>
                  <div className="text-sm text-gray-500 dark:text-gray-400">
                    {settings?.transcription.provider === 'local'
                      ? `Whisper: ${settings?.transcription.local.model || 'base'}`
                      : 'Groq API'}
                  </div>
                </div>
                <button
                  onClick={() => setView('settings')}
                  className="text-blue-600 hover:text-blue-700 text-sm font-medium"
                >
                  Configure
                </button>
              </div>
            </div>
          </div>
        )}

        {view === 'history' && <HistoryPanel />}
        {view === 'settings' && <SettingsPanel />}
      </main>

      {/* Popup overlay for showing transcription when no text field is active */}
      <PopupOverlay />

      {/* Onboarding for new users */}
      {showOnboarding && <Onboarding onComplete={handleOnboardingComplete} />}
    </div>
  );
}

export default App;
