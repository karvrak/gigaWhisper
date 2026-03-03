import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { SettingsPanel } from './components/SettingsPanel';
import { PopupOverlay } from './components/PopupOverlay';
import { HistoryPanel } from './components/HistoryPanel';
import { Onboarding } from './components/Onboarding';
import { UpdateNotification } from './components/UpdateNotification';
import { useSettings } from './hooks/useSettings';
import { Home, Clock, Settings, Mic, Cpu, Cloud, ChevronRight, Zap, X } from 'lucide-react';

const ONBOARDING_KEY = 'gigawhisper_onboarding_completed';
const PRO_BANNER_DISMISSED_KEY = 'gigawhisper_pro_banner_dismissed';

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
  const [proBannerDismissed, setProBannerDismissed] = useState(() => {
    return localStorage.getItem(PRO_BANNER_DISMISSED_KEY) === 'true';
  });

  const handleOnboardingComplete = () => {
    localStorage.setItem(ONBOARDING_KEY, 'true');
    setShowOnboarding(false);
  };

  const dismissProBanner = () => {
    localStorage.setItem(PRO_BANNER_DISMISSED_KEY, 'true');
    setProBannerDismissed(true);
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
      <div className="flex items-center justify-center h-screen bg-[#f8f7fc] dark:bg-[#0f0d1a]">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-sm text-gray-500 dark:text-indigo-300/50">Loading...</span>
        </div>
      </div>
    );
  }

  const navItems = [
    { id: 'main' as const, label: 'Home', icon: Home },
    { id: 'history' as const, label: 'History', icon: Clock },
    { id: 'settings' as const, label: 'Settings', icon: Settings },
  ];

  return (
    <div className="h-screen flex flex-col bg-[#f8f7fc] dark:bg-[#0f0d1a] text-gray-900 dark:text-gray-100 overflow-hidden">
      {/* Navigation Bar */}
      <header className="bg-white/80 dark:bg-[#1a1725]/80 backdrop-blur-xl border-b border-indigo-100/80 dark:border-violet-500/10 select-none flex-shrink-0">
        <div className="flex items-center h-11">
          {/* Logo - left side */}
          <div className="w-12 h-11 flex items-center justify-center">
            <img src="/icon.ico" alt="GigaWhisper" className="w-5 h-5 opacity-90" />
          </div>

          {/* Navigation - centered */}
          <nav className="flex-1 flex justify-center gap-1">
            {navItems.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                onClick={() => setView(id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-150 ${
                  view === id
                    ? 'bg-gradient-to-r from-indigo-500/10 to-violet-500/10 text-indigo-600 dark:text-indigo-400 shadow-sm'
                    : 'text-gray-600 dark:text-gray-400 hover:bg-indigo-50 dark:hover:bg-indigo-500/5 hover:text-gray-900 dark:hover:text-gray-200'
                }`}
              >
                <Icon className="w-4 h-4" />
                <span>{label}</span>
              </button>
            ))}
          </nav>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto overflow-x-hidden p-5">
        {view === 'main' && (
          <div className="space-y-4 max-w-xl mx-auto animate-fade-in">
            {/* Status Indicator */}
            <div className="flex items-center justify-center gap-2 py-1">
              <div className="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.5)]" />
              <span className="text-xs font-medium text-emerald-600 dark:text-emerald-400">Ready</span>
            </div>

            {/* Welcome Card */}
            <div className="card p-5 hover-lift cursor-pointer group hover:border-indigo-300/30 dark:hover:border-indigo-500/20 transition-all" onClick={() => setView('settings')}>
              <div className="flex items-center gap-4">
                <div className="p-2.5 rounded-xl bg-indigo-50 dark:bg-indigo-500/10">
                  <Mic className="w-5 h-5 text-indigo-500" />
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
                <ChevronRight className="w-5 h-5 text-gray-400 group-hover:text-indigo-400 transition-colors" />
              </div>
            </div>

            {/* Provider Card */}
            <div className="card p-5 hover-lift cursor-pointer group hover:border-indigo-300/30 dark:hover:border-indigo-500/20 transition-all" onClick={() => setView('settings')}>
              <div className="flex items-center gap-4">
                <div className={`p-2.5 rounded-xl ${
                  settings?.transcription.provider === 'local'
                    ? 'bg-emerald-50 dark:bg-emerald-500/10'
                    : 'bg-violet-50 dark:bg-violet-500/10'
                }`}>
                  {settings?.transcription.provider === 'local' ? (
                    <Cpu className="w-5 h-5 text-emerald-500" />
                  ) : (
                    <Cloud className="w-5 h-5 text-violet-500" />
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
                <ChevronRight className="w-5 h-5 text-gray-400 group-hover:text-indigo-400 transition-colors" />
              </div>
            </div>

            {/* Quick tip */}
            <div className="text-center pt-2">
              <p className="text-xs text-gray-400 dark:text-indigo-300/30">
                GigaWhisper runs in your system tray. Use the shortcut anywhere to transcribe.
              </p>
            </div>

            {/* Pro Upsell Banner */}
            {!proBannerDismissed && !showOnboarding && (
              <div className="relative mt-4 p-4 rounded-xl bg-gradient-to-r from-indigo-500/5 to-violet-500/5 dark:from-indigo-900/30 dark:to-violet-900/30 border border-indigo-200/50 dark:border-violet-500/15 animate-fade-in">
                <button
                  onClick={dismissProBanner}
                  className="absolute top-2 right-2 p-1 rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-white/5 transition-colors"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
                <div className="flex items-center gap-3 pr-6">
                  <div className="p-2 rounded-lg bg-gradient-to-br from-indigo-500 to-violet-500 text-white flex-shrink-0">
                    <Zap className="w-4 h-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="text-sm font-semibold text-gray-900 dark:text-indigo-200">GigaWhisper Pro</h3>
                    <p className="text-xs text-gray-500 dark:text-indigo-300/50">Unlimited cloud, priority processing</p>
                  </div>
                  <a
                    href="https://gigawhisper.com/pro"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex-shrink-0 px-3 py-1.5 text-xs font-medium text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 border border-indigo-200 dark:border-indigo-500/30 rounded-lg hover:bg-indigo-50 dark:hover:bg-indigo-500/10 transition-colors"
                  >
                    Learn more
                  </a>
                </div>
              </div>
            )}
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
