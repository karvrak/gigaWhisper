import { usePremium } from '../../hooks/usePremium';
import { THEME_PRESETS } from '../../themes/presets';
import { PremiumBadge } from './PremiumBadge';
import { Check } from 'lucide-react';
import type { ThemeDefinition } from '../../themes/types';

interface ThemeSelectorProps {
  currentTheme: string | null;
  onChange: (themeId: string | null) => void;
}

/** Convert hex color (#RRGGBB) to space-separated RGB values (e.g. "26 23 37") */
function hexToRgb(hex: string): string {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16);
  const g = parseInt(h.substring(2, 4), 16);
  const b = parseInt(h.substring(4, 6), 16);
  return `${r} ${g} ${b}`;
}

function applyCustomTheme(theme: ThemeDefinition | null) {
  const root = document.documentElement;
  if (!theme) {
    // Remove custom properties — restore defaults by unsetting overrides
    root.removeAttribute('data-custom-theme');
    const props = [
      '--color-bg-primary', '--color-bg-secondary', '--color-bg-tertiary',
      '--color-text-primary', '--color-text-secondary', '--color-text-tertiary', '--color-text-muted',
      '--color-border-default', '--color-border-hover',
      '--color-accent-primary', '--color-accent-hover', '--color-accent-subtle',
      '--color-success', '--color-error',
    ];
    props.forEach((p) => root.style.removeProperty(p));
    return;
  }

  root.setAttribute('data-custom-theme', theme.id);
  // All premium themes are dark — ensure dark class is applied
  root.classList.add('dark');
  const { colors } = theme;
  // Map theme colors to the CSS custom properties used by globals.css (RGB values)
  root.style.setProperty('--color-bg-primary', hexToRgb(colors.bgPrimary));
  root.style.setProperty('--color-bg-secondary', hexToRgb(colors.bgSecondary));
  root.style.setProperty('--color-bg-tertiary', hexToRgb(colors.bgInput));
  root.style.setProperty('--color-text-primary', hexToRgb(colors.textPrimary));
  root.style.setProperty('--color-text-secondary', hexToRgb(colors.textSecondary));
  root.style.setProperty('--color-text-tertiary', hexToRgb(colors.textMuted));
  root.style.setProperty('--color-text-muted', hexToRgb(colors.textMuted));
  root.style.setProperty('--color-border-default', hexToRgb(colors.borderDefault));
  root.style.setProperty('--color-border-hover', hexToRgb(colors.borderHover));
  root.style.setProperty('--color-accent-primary', hexToRgb(colors.accentPrimary));
  root.style.setProperty('--color-accent-hover', hexToRgb(colors.accentHover));
  root.style.setProperty('--color-accent-subtle', hexToRgb(colors.bgCard));
  root.style.setProperty('--color-success', hexToRgb(colors.success));
  root.style.setProperty('--color-error', hexToRgb(colors.error));
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
                ? 'border-accent shadow-md'
                : 'border-edge hover:border-edge-hover'
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
              <div className="absolute -top-1 -right-1 w-4 h-4 bg-accent rounded-full flex items-center justify-center">
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
