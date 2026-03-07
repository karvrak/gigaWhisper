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
  // Don't change dark/light class if a custom theme is active (they force dark)
  if (root.hasAttribute('data-custom-theme')) return;
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
      <div className="flex items-center justify-center h-screen bg-surface-primary">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-2 border-accent border-t-transparent rounded-full animate-spin" />
          <span className="text-sm text-content-tertiary">Loading...</span>
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
    <div className="h-screen flex flex-col bg-surface-primary text-content-primary overflow-hidden">
      {/* Navigation Bar */}
      <header className="bg-surface-secondary/80 backdrop-blur-xl border-b border-edge select-none flex-shrink-0">
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
                    ? 'bg-accent/10 text-accent shadow-sm'
                    : 'text-content-secondary hover:bg-accent/5 hover:text-content-primary'
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
            <div className="card p-5 hover-lift cursor-pointer group hover:border-accent/20 transition-all" onClick={() => setView('settings')}>
              <div className="flex items-center gap-4">
                <div className="p-2.5 rounded-xl bg-accent/10">
                  <Mic className="w-5 h-5 text-accent" />
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-base font-semibold mb-1">
                    Keyboard Shortcut
                  </h2>
                  <p className="text-sm text-content-secondary mb-3">
                    {settings?.recording.mode === 'push-to-talk'
                      ? 'Hold the shortcut to record, release to transcribe'
                      : 'Press once to start, press again to stop recording'}
                  </p>
                  <kbd className="inline-flex items-center px-3 py-1.5 text-xs font-medium">
                    {settings?.shortcuts.record || 'Ctrl+Shift+Space'}
                  </kbd>
                </div>
                <ChevronRight className="w-5 h-5 text-content-tertiary group-hover:text-accent transition-colors" />
              </div>
            </div>

            {/* Provider Card */}
            <div className="card p-5 hover-lift cursor-pointer group hover:border-accent/20 transition-all" onClick={() => setView('settings')}>
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
                  <h2 className="text-base font-semibold mb-0.5">
                    Transcription Provider
                  </h2>
                  <p className="text-sm text-content-secondary">
                    {settings?.transcription.provider === 'local' ? (
                      <>Local - Whisper {settings?.transcription.local.model || 'base'}</>
                    ) : (
                      <>Cloud - Groq API</>
                    )}
                  </p>
                </div>
                <ChevronRight className="w-5 h-5 text-content-tertiary group-hover:text-accent transition-colors" />
              </div>
            </div>

            {/* Quick tip */}
            <div className="text-center pt-2">
              <p className="text-xs text-content-muted">
                GigaWhisper runs in your system tray. Use the shortcut anywhere to transcribe.
              </p>
            </div>

            {/* Pro Upsell Banner */}
            {!proBannerDismissed && !showOnboarding && (
              <div className="relative mt-4 p-4 rounded-xl bg-accent/5 border border-edge animate-fade-in">
                <button
                  onClick={dismissProBanner}
                  className="absolute top-2 right-2 p-1 rounded-md text-content-tertiary hover:text-content-primary hover:bg-surface-tertiary transition-colors"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
                <div className="flex items-center gap-3 pr-6">
                  <div className="p-2 rounded-lg bg-gradient-to-br from-indigo-500 to-violet-500 text-white flex-shrink-0">
                    <Zap className="w-4 h-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="text-sm font-semibold">GigaWhisper Pro</h3>
                    <p className="text-xs text-content-secondary">Unlimited cloud, priority processing</p>
                  </div>
                  <a
                    href="https://gigawhisper.com/pro"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex-shrink-0 px-3 py-1.5 text-xs font-medium text-accent hover:text-accent-hover border border-edge rounded-lg hover:bg-accent/5 transition-colors"
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
