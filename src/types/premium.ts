// Premium feature types

export interface PremiumStatus {
  is_premium: boolean;
  expires_at: number | null;
  last_validated: number | null;
  credits_balance: number;
  credits_low: boolean;
  features: FeatureStatus[];
}

export interface FeatureStatus {
  id: string;
  name: string;
  available: boolean;
}

export type PremiumFeature =
  | 'custom-themes'
  | 'multi-context'
  | 'llm-post-processing'
  | 'openai-provider'
  | 'deepgram-provider'
  | 'custom-provider';

export interface TranscriptionContext {
  id: string;
  name: string;
  shortcut: string;
  language: string;
  provider: 'local' | 'groq' | 'openai' | 'deepgram' | 'custom';
  model: string | null;
  post_processing: ContextPostProcessing | null;
  color: string | null;
  icon: string | null;
  custom_vocabulary: string | null;
  app_patterns: string[];
}

export interface ContextPostProcessing {
  enabled: boolean;
  llm_provider: 'open-ai' | 'anthropic' | 'groq-llm' | 'custom-llm';
  system_prompt: string;
}

export interface CreditState {
  balance_eur: number;
  total_used_eur: number;
  low_threshold_eur: number;
  last_synced: number | null;
}
