import { Cpu, Cloud, Zap, AudioWaveform } from 'lucide-react';
import { PremiumBadge } from './premium/PremiumBadge';

type Provider = 'local' | 'groq' | 'openai' | 'deepgram';

interface ProviderToggleProps {
  value: Provider;
  onChange: (value: Provider) => void;
  isPremium?: boolean;
}

const providers = [
  {
    id: 'local' as const,
    label: 'Local',
    icon: Cpu,
    description: 'Run whisper.cpp on your machine. Private, works offline.',
    premium: false,
  },
  {
    id: 'groq' as const,
    label: 'Groq Cloud',
    icon: Zap,
    description: 'Fast cloud transcription via Groq. Requires API key.',
    premium: false,
  },
  {
    id: 'openai' as const,
    label: 'OpenAI',
    icon: Cloud,
    description: 'OpenAI Whisper API. High quality, requires API key.',
    premium: true,
  },
  {
    id: 'deepgram' as const,
    label: 'Deepgram',
    icon: AudioWaveform,
    description: 'Deepgram Nova-2. Fast and accurate, requires API key.',
    premium: true,
  },
];

export function ProviderToggle({ value, onChange, isPremium = false }: ProviderToggleProps) {
  return (
    <div className="grid grid-cols-2 gap-3">
      {providers.map(({ id, label, icon: Icon, description, premium }) => {
        const locked = premium && !isPremium;
        return (
          <button
            key={id}
            onClick={() => !locked && onChange(id)}
            disabled={locked}
            className={`p-4 border rounded-lg text-left transition-colors ${
              value === id
                ? 'border-accent bg-accent-subtle'
                : locked
                  ? 'border-edge opacity-60 cursor-not-allowed'
                  : 'border-edge hover:border-edge-hover'
            }`}
          >
            <div className="flex items-center gap-3 mb-2">
              <div
                className={`p-2 rounded-lg ${
                  value === id
                    ? 'bg-accent/10 text-accent'
                    : 'bg-surface-tertiary text-content-secondary'
                }`}
              >
                <Icon className="w-5 h-5" />
              </div>
              <div className="font-medium">{label}</div>
              {premium && <PremiumBadge locked={!isPremium} />}
            </div>
            <p className="text-sm text-content-secondary">
              {description}
            </p>
          </button>
        );
      })}
    </div>
  );
}
