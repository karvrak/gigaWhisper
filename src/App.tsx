import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SettingsPanel } from './components/SettingsPanel';
import { PopupOverlay } from './components/PopupOverlay';
import { HistoryPanel } from './components/HistoryPanel';
import { Onboarding } from './components/Onboarding';
import { UpdateNotification } from './components/UpdateNotification';
import { useSettings } from './hooks/useSettings';
import { Minus, X, Home, Clock, Settings, Mic, Cpu, Cloud, ChevronRight } from 'lucide-react';

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
      <div className="flex items-center justify-center h-screen bg-gray-50 dark:bg-gray-900">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-sm text-gray-500 dark:text-gray-400">Loading...</span>
        </div>
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

  const navItems = [
    { id: 'main' as const, label: 'Home', icon: Home },
    { id: 'history' as const, label: 'History', icon: Clock },
    { id: 'settings' as const, label: 'Settings', icon: Settings },
  ];

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100 overflow-hidden">
      {/* Custom Titlebar */}
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200/80 dark:border-gray-700/80 select-none flex-shrink-0">
        <div className="flex items-center h-11">
          {/* Logo - left side */}
          <div className="w-12 h-11 flex items-center justify-center" data-tauri-drag-region>
            <img src="/icon.ico" alt="GigaWhisper" className="w-5 h-5 opacity-90" />
          </div>

          {/* Navigation - centered */}
          <nav className="flex-1 flex justify-center gap-1" data-tauri-drag-region>
            {navItems.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                onClick={() => setView(id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-150 ${
                  view === id
                    ? 'bg-blue-50 text-blue-600 dark:bg-blue-500/10 dark:text-blue-400'
                    : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-gray-200'
                }`}
              >
                <Icon className="w-4 h-4" />
                <span>{label}</span>
              </button>
            ))}
          </nav>

          {/* Window Controls */}
          <div className="flex" style={{ WebkitAppRegion: 'no-drag' } as React.CSSProperties}>
            <button
              onClick={handleMinimize}
              onMouseDown={(e) => e.stopPropagation()}
              className="w-11 h-11 flex items-center justify-center text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700/50 transition-colors duration-150"
              title="Minimize"
            >
              <Minus className="w-4 h-4 pointer-events-none" />
            </button>
            <button
              onClick={handleClose}
              onMouseDown={(e) => e.stopPropagation()}
              className="w-11 h-11 flex items-center justify-center text-gray-500 hover:bg-red-500 hover:text-white transition-colors duration-150"
              title="Close"
            >
              <X className="w-4 h-4 pointer-events-none" />
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto overflow-x-hidden p-5">
        {view === 'main' && (
          <div className="space-y-4 max-w-xl mx-auto animate-fade-in">
            {/* Welcome Card */}
            <div className="card p-5">
              <div className="flex items-start gap-4">
                <div className="p-2.5 rounded-xl bg-blue-50 dark:bg-blue-500/10">
                  <Mic className="w-5 h-5 text-blue-500" />
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-base font-semibold text-gray-900 dark:text-gray-100 mb-1">
                    Keyboard Shortcut
                  </h2>
                  <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">
                    {settings?.recording.mode === 'push-to-talk'
                      ? 'Hold the shortcut to record, release to transcribe'
                      : 'Press once to start, press again to stop recording'}
                  </p>
                  <kbd className="inline-flex items-center px-3 py-1.5 text-xs font-medium">
                    {settings?.shortcuts.record || 'Ctrl+Shift+Space'}
                  </kbd>
                </div>
              </div>
            </div>

            {/* Provider Card */}
            <div className="card p-5 hover-lift cursor-pointer" onClick={() => setView('settings')}>
              <div className="flex items-center gap-4">
                <div className={`p-2.5 rounded-xl ${
                  settings?.transcription.provider === 'local'
                    ? 'bg-emerald-50 dark:bg-emerald-500/10'
                    : 'bg-purple-50 dark:bg-purple-500/10'
                }`}>
                  {settings?.transcription.provider === 'local' ? (
                    <Cpu className="w-5 h-5 text-emerald-500" />
                  ) : (
                    <Cloud className="w-5 h-5 text-purple-500" />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-base font-semibold text-gray-900 dark:text-gray-100 mb-0.5">
                    Transcription Provider
                  </h2>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    {settings?.transcription.provider === 'local' ? (
                      <>Local - Whisper {settings?.transcription.local.model || 'base'}</>
                    ) : (
                      <>Cloud - Groq API</>
                    )}
                  </p>
                </div>
                <ChevronRight className="w-5 h-5 text-gray-400" />
              </div>
            </div>

            {/* Quick tip */}
            <div className="text-center pt-2">
              <p className="text-xs text-gray-400 dark:text-gray-500">
                GigaWhisper runs in your system tray. Use the shortcut anywhere to transcribe.
              </p>
            </div>
          </div>
        )}

        {view === 'history' && <HistoryPanel />}
        {view === 'settings' && <SettingsPanel />}
      </main>

      {/* Popup overlay for showing transcription when no text field is active */}
      <PopupOverlay />

      {/* Update notification */}
      <UpdateNotification />

      {/* Onboarding for new users */}
      {showOnboarding && <Onboarding onComplete={handleOnboardingComplete} />}
    </div>
  );
}

export default App;
