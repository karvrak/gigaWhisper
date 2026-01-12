import { Cpu, Cloud } from 'lucide-react';

type Provider = 'local' | 'groq';

interface ProviderToggleProps {
  value: Provider;
  onChange: (value: Provider) => void;
}

export function ProviderToggle({ value, onChange }: ProviderToggleProps) {
  return (
    <div className="flex gap-4">
      {/* Local Option */}
      <button
        onClick={() => onChange('local')}
        className={`flex-1 p-4 border rounded-lg text-left transition-colors ${
          value === 'local'
            ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
            : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
        }`}
      >
        <div className="flex items-center gap-3 mb-2">
          <div
            className={`p-2 rounded-lg ${
              value === 'local'
                ? 'bg-blue-100 text-blue-600 dark:bg-blue-800 dark:text-blue-300'
                : 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400'
            }`}
          >
            <Cpu className="w-5 h-5" />
          </div>
          <div className="font-medium">Local</div>
        </div>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          Run whisper.cpp on your machine. Private, works offline.
        </p>
      </button>

      {/* Cloud Option */}
      <button
        onClick={() => onChange('groq')}
        className={`flex-1 p-4 border rounded-lg text-left transition-colors ${
          value === 'groq'
            ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
            : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
        }`}
      >
        <div className="flex items-center gap-3 mb-2">
          <div
            className={`p-2 rounded-lg ${
              value === 'groq'
                ? 'bg-blue-100 text-blue-600 dark:bg-blue-800 dark:text-blue-300'
                : 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400'
            }`}
          >
            <Cloud className="w-5 h-5" />
          </div>
          <div className="font-medium">Groq Cloud</div>
        </div>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          Fast cloud transcription. Best quality, requires API key.
        </p>
      </button>
    </div>
  );
}
