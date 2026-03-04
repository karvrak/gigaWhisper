import { usePremium } from '../../hooks/usePremium';
import { useSettings } from '../../hooks/useSettings';
import { PremiumBadge } from './PremiumBadge';
import { Sparkles } from 'lucide-react';

export function PostProcessingConfig() {
  const { settings, updateSettings } = useSettings();
  const { isFeatureAvailable } = usePremium();
  const canUse = isFeatureAvailable('llm-post-processing');

  if (!settings) return null;

  const pp = settings.post_processing;

  const updatePP = (updates: Partial<typeof pp>) => {
    updateSettings({
      ...settings,
      post_processing: { ...pp, ...updates },
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-violet-500" />
        <h3 className="text-sm font-medium">AI Post-Processing</h3>
        <PremiumBadge feature="llm-post-processing" />
      </div>

      <p className="text-xs text-gray-500 dark:text-gray-400">
        Use AI to clean up, reformulate, or adapt transcriptions after they are generated.
      </p>

      <div className="flex items-start gap-3">
        <input
          type="checkbox"
          id="pp-enabled"
          checked={pp.enabled}
          onChange={(e) => updatePP({ enabled: e.target.checked })}
          disabled={!canUse}
          className="rounded text-indigo-600 mt-0.5"
        />
        <div>
          <label htmlFor="pp-enabled" className="font-medium text-sm cursor-pointer">
            Enable post-processing
          </label>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Process transcriptions through an LLM before output
          </p>
        </div>
      </div>

      {pp.enabled && (
        <>
          <div>
            <label className="block text-xs font-medium mb-1">LLM Provider</label>
            <select
              value={pp.default_provider}
              onChange={(e) => updatePP({ default_provider: e.target.value as 'open-ai' | 'anthropic' | 'groq-llm' })}
              disabled={!canUse}
              className="w-full px-3 py-2 border border-gray-300 dark:border-violet-500/20 rounded-md bg-white dark:bg-[#252136] text-sm"
            >
              <option value="groq-llm">Groq (Llama - Fast)</option>
              <option value="open-ai">OpenAI (GPT-4o-mini)</option>
              <option value="anthropic">Anthropic (Claude Haiku)</option>
            </select>
          </div>

          <div>
            <label className="block text-xs font-medium mb-1">System Prompt</label>
            <textarea
              value={pp.default_prompt}
              onChange={(e) => updatePP({ default_prompt: e.target.value })}
              disabled={!canUse}
              rows={4}
              placeholder="Instructions for the AI on how to process the transcription..."
              className="w-full px-3 py-2 border border-gray-300 dark:border-violet-500/20 rounded-md bg-white dark:bg-[#252136] text-sm resize-none"
            />
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              Customize how the AI processes your transcriptions
            </p>
          </div>
        </>
      )}
    </div>
  );
}
