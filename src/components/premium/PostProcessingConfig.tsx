import { usePremium } from '../../hooks/usePremium';
import { useSettings } from '../../hooks/useSettings';
import { PremiumBadge } from './PremiumBadge';
import { ApiKeyInput } from './ApiKeyInput';
import { Sparkles, AlertTriangle } from 'lucide-react';

export function PostProcessingConfig() {
  const { settings, updateSettings, resetSettings } = useSettings();
  const { isFeatureAvailable } = usePremium();
  const canUse = isFeatureAvailable('llm-post-processing');

  if (!settings) return null;

  const pp = settings.post_processing;

  const hasAnyLlmProvider =
    settings.transcription.groq.api_key_configured ||
    pp.openai.api_key_configured ||
    pp.anthropic.api_key_configured;

  const updatePP = (updates: Partial<typeof pp>) => {
    updateSettings({
      ...settings,
      post_processing: { ...pp, ...updates },
    });
  };

  const handleKeyStatusChange = () => {
    // Reload settings to get updated api_key_configured flags
    resetSettings();
  };

  const navigateToTranscription = () => {
    const tabSelect = document.querySelector('select') as HTMLSelectElement;
    if (tabSelect) {
      tabSelect.value = 'transcription';
      tabSelect.dispatchEvent(new Event('change', { bubbles: true }));
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-violet-500" />
        <h3 className="text-sm font-medium">AI Post-Processing</h3>
        <PremiumBadge feature="llm-post-processing" />
      </div>

      <p className="text-xs text-content-secondary">
        Use AI to clean up, reformulate, or adapt transcriptions after they are generated.
      </p>

      {!hasAnyLlmProvider ? (
        <div
          className="flex items-start gap-2 p-2 bg-amber-500/10 border border-amber-500/20 rounded-md cursor-pointer hover:bg-amber-500/15 transition-colors"
          onClick={navigateToTranscription}
        >
          <AlertTriangle className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
          <div>
            <p className="text-xs text-amber-200">No LLM provider configured</p>
            <p className="text-xs text-content-tertiary mt-0.5">
              Click here to configure an API key (Groq, OpenAI, or Anthropic) in the Transcription settings.
            </p>
          </div>
        </div>
      ) : (
        <>
          <div className="flex items-start gap-3">
            <input
              type="checkbox"
              id="pp-enabled"
              checked={pp.enabled}
              onChange={(e) => updatePP({ enabled: e.target.checked })}
              disabled={!canUse}
              className="rounded mt-0.5"
            />
            <div>
              <label htmlFor="pp-enabled" className="font-medium text-sm cursor-pointer">
                Enable post-processing
              </label>
              <p className="text-xs text-content-secondary">
                Process transcriptions through an LLM before output
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <input
              type="checkbox"
              id="pp-filler-words"
              checked={pp.remove_filler_words}
              onChange={(e) => updatePP({ remove_filler_words: e.target.checked })}
              disabled={!canUse}
              className="rounded mt-0.5"
            />
            <label htmlFor="pp-filler-words" className="text-xs text-content-secondary cursor-pointer">
              Remove filler words (um, uh, like, you know...)
            </label>
          </div>

          {pp.enabled && (
            <>
              <div>
                <label className="block text-xs font-medium mb-1">LLM Provider</label>
                <select
                  value={pp.default_provider}
                  onChange={(e) => updatePP({ default_provider: e.target.value as 'open-ai' | 'anthropic' | 'groq-llm' })}
                  disabled={!canUse}
                  className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                >
                  <option value="groq-llm">Groq (Llama - Fast)</option>
                  <option value="open-ai">OpenAI (GPT-4o-mini)</option>
                  <option value="anthropic">Anthropic (Claude Haiku)</option>
                </select>
              </div>

              {/* API Key for selected provider */}
              {pp.default_provider === 'open-ai' && (
                <ApiKeyInput
                  provider="OpenAI"
                  label="OpenAI API Key"
                  placeholder="sk-..."
                  isConfigured={pp.openai.api_key_configured}
                  setCommand="set_openai_api_key"
                  clearCommand="clear_openai_api_key"
                  validateCommand="validate_openai_api_key_live"
                  onStatusChange={handleKeyStatusChange}
                />
              )}

              {pp.default_provider === 'anthropic' && (
                <ApiKeyInput
                  provider="Anthropic"
                  label="Anthropic API Key"
                  placeholder="sk-ant-..."
                  isConfigured={pp.anthropic.api_key_configured}
                  setCommand="set_anthropic_api_key"
                  clearCommand="clear_anthropic_api_key"
                  validateCommand="validate_anthropic_api_key_live"
                  onStatusChange={handleKeyStatusChange}
                />
              )}

              {pp.default_provider === 'groq-llm' && (
                <p className="text-xs text-content-secondary italic">
                  Groq uses the same API key as transcription (configured in Transcription settings).
                </p>
              )}

              <div>
                <label className="block text-xs font-medium mb-1">System Prompt</label>
                <textarea
                  value={pp.default_prompt}
                  onChange={(e) => updatePP({ default_prompt: e.target.value })}
                  disabled={!canUse}
                  rows={4}
                  placeholder="Instructions for the AI on how to process the transcription..."
                  className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm resize-none"
                />
                <p className="mt-1 text-xs text-content-secondary">
                  Customize how the AI processes your transcriptions
                </p>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
