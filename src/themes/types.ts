export interface ThemeDefinition {
  id: string;
  name: string;
  premium: boolean;
  colors: ThemeColors;
}

export interface ThemeColors {
  // Background
  bgPrimary: string;
  bgSecondary: string;
  bgCard: string;
  bgInput: string;

  // Text
  textPrimary: string;
  textSecondary: string;
  textMuted: string;

  // Accent
  accentPrimary: string;
  accentSecondary: string;
  accentHover: string;

  // Border
  borderDefault: string;
  borderHover: string;

  // Status
  success: string;
  warning: string;
  error: string;
}
