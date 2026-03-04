import { usePremium } from '../../hooks/usePremium';
import { THEME_PRESETS } from '../../themes/presets';
import { PremiumBadge } from './PremiumBadge';
import { Check } from 'lucide-react';
import type { ThemeDefinition } from '../../themes/types';

interface ThemeSelectorProps {
  currentTheme: string | null;
  onChange: (themeId: string | null) => void;
}

function applyCustomTheme(theme: ThemeDefinition | null) {
  const root = document.documentElement;
  if (!theme) {
    // Remove custom properties
    root.removeAttribute('data-custom-theme');
    return;
  }

  root.setAttribute('data-custom-theme', theme.id);
  const { colors } = theme;
  root.style.setProperty('--bg-primary', colors.bgPrimary);
  root.style.setProperty('--bg-secondary', colors.bgSecondary);
  root.style.setProperty('--bg-card', colors.bgCard);
  root.style.setProperty('--bg-input', colors.bgInput);
  root.style.setProperty('--text-primary', colors.textPrimary);
  root.style.setProperty('--text-secondary', colors.textSecondary);
  root.style.setProperty('--text-muted', colors.textMuted);
  root.style.setProperty('--accent-primary', colors.accentPrimary);
  root.style.setProperty('--accent-secondary', colors.accentSecondary);
  root.style.setProperty('--accent-hover', colors.accentHover);
  root.style.setProperty('--border-default', colors.borderDefault);
  root.style.setProperty('--border-hover', colors.borderHover);
  root.style.setProperty('--color-success', colors.success);
  root.style.setProperty('--color-warning', colors.warning);
  root.style.setProperty('--color-error', colors.error);
}

export function ThemeSelector({ currentTheme, onChange }: ThemeSelectorProps) {
  const { isFeatureAvailable } = usePremium();
  const canUseThemes = isFeatureAvailable('custom-themes');

  const handleSelect = (theme: ThemeDefinition) => {
    if (!canUseThemes) return;
    const newTheme = currentTheme === theme.id ? null : theme.id;
    onChange(newTheme);
    applyCustomTheme(newTheme ? theme : null);
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <label className="text-sm font-medium">Premium Themes</label>
        <PremiumBadge feature="custom-themes" />
      </div>
      <div className="grid grid-cols-5 gap-2">
        {THEME_PRESETS.map((theme) => (
          <button
            key={theme.id}
            onClick={() => handleSelect(theme)}
            disabled={!canUseThemes}
            className={`relative p-2 rounded-lg border-2 transition-all text-center ${
              currentTheme === theme.id
                ? 'border-indigo-500 shadow-md'
                : 'border-gray-200 dark:border-violet-500/15 hover:border-gray-300'
            } ${!canUseThemes ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
          >
            {/* Color preview */}
            <div className="flex gap-0.5 justify-center mb-1.5">
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: theme.colors.accentPrimary }} />
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: theme.colors.accentSecondary }} />
              <div className="w-3 h-3 rounded-full" style={{ backgroundColor: theme.colors.bgPrimary }} />
            </div>
            <span className="text-xs font-medium">{theme.name}</span>
            {currentTheme === theme.id && (
              <div className="absolute -top-1 -right-1 w-4 h-4 bg-indigo-500 rounded-full flex items-center justify-center">
                <Check className="w-3 h-3 text-white" />
              </div>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}

export { applyCustomTheme };
