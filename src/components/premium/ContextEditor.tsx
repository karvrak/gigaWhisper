import { useState } from 'react';
import { HotkeyInput } from '../HotkeyInput';
import { Save, ArrowLeft, Sparkles } from 'lucide-react';
import type { TranscriptionContext, ContextPostProcessing } from '../../types/premium';

interface ProviderAvailability {
  groq: boolean;
  openai: boolean;
  deepgram: boolean;
}

interface ContextEditorProps {
  context: TranscriptionContext;
  onSave: (ctx: TranscriptionContext) => Promise<void>;
  onCancel: () => void;
  providerAvailability?: ProviderAvailability;
}

export function ContextEditor({ context, onSave, onCancel, providerAvailability }: ContextEditorProps) {
  const pa = providerAvailability ?? { groq: true, openai: true, deepgram: true };
  const [ctx, setCtx] = useState(context);
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!ctx.name.trim()) return;
    setSaving(true);
    try {
      await onSave(ctx);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <button onClick={onCancel} className="p-1 hover:bg-surface-tertiary rounded">
          <ArrowLeft className="w-4 h-4" />
        </button>
        <h3 className="text-sm font-medium">{!context.name ? 'New Context' : `Edit: ${context.name}`}</h3>
      </div>

      <div className="space-y-3">
        <div>
          <label className="block text-xs font-medium mb-1">Name</label>
          <input
            type="text"
            value={ctx.name}
            onChange={(e) => setCtx({ ...ctx, name: e.target.value })}
            placeholder="e.g., Dev, Team, Personal"
            className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
          />
        </div>

        <div>
          <label className="block text-xs font-medium mb-1">Shortcut</label>
          <HotkeyInput
            value={ctx.shortcut}
            onChange={(shortcut) => setCtx({ ...ctx, shortcut })}
          />
        </div>

        <div>
          <label className="block text-xs font-medium mb-1">Language</label>
          <select
            value={ctx.language}
            onChange={(e) => setCtx({ ...ctx, language: e.target.value })}
            className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
          >
            <option value="auto">Auto-detect</option>
            <option value="en">English</option>
            <option value="fr">French</option>
            <option value="de">German</option>
            <option value="es">Spanish</option>
            <option value="it">Italian</option>
            <option value="pt">Portuguese</option>
            <option value="ja">Japanese</option>
            <option value="ko">Korean</option>
            <option value="zh">Chinese</option>
          </select>
        </div>

        <div>
          <label className="block text-xs font-medium mb-1">Provider</label>
          <select
            value={ctx.provider}
            onChange={(e) => setCtx({ ...ctx, provider: e.target.value as TranscriptionContext['provider'] })}
            className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
          >
            <option value="local">Local (Whisper)</option>
            <option value="groq" disabled={!pa.groq}>
              Groq Cloud{!pa.groq ? ' (No API key)' : ''}
            </option>
            <option value="openai" disabled={!pa.openai}>
              OpenAI Whisper{!pa.openai ? ' (No API key)' : ''}
            </option>
            <option value="deepgram" disabled={!pa.deepgram}>
              Deepgram Nova{!pa.deepgram ? ' (No API key)' : ''}
            </option>
          </select>
        </div>

        <div className="flex gap-4">
          <div>
            <label className="block text-xs font-medium mb-1">Icon (emoji)</label>
            <input
              type="text"
              value={ctx.icon || ''}
              onChange={(e) => setCtx({ ...ctx, icon: e.target.value || null })}
              placeholder="e.g., 💻"
              maxLength={2}
              className="w-20 px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm text-center"
            />
          </div>
          <div>
            <label className="block text-xs font-medium mb-1">Indicator Color</label>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={ctx.color || '#818cf8'}
                onChange={(e) => setCtx({ ...ctx, color: e.target.value })}
                className="w-10 h-10 rounded-md cursor-pointer border border-edge bg-transparent p-0.5"
              />
              {ctx.color && (
                <button
                  type="button"
                  onClick={() => setCtx({ ...ctx, color: null })}
                  className="text-xs text-content-tertiary hover:text-content-secondary"
                >
                  Reset
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Post-Processing */}
        <div className="border border-edge rounded-lg p-3 space-y-3">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-violet-500" />
            <span className="text-xs font-medium">AI Post-Processing</span>
          </div>

          <div className="flex items-start gap-3">
            <input
              type="checkbox"
              id="ctx-pp-enabled"
              checked={ctx.post_processing?.enabled ?? false}
              onChange={(e) => {
                const pp: ContextPostProcessing = ctx.post_processing ?? {
                  enabled: false,
                  llm_provider: 'groq-llm',
                  system_prompt: 'Clean up and fix any errors in this transcription. Keep the original meaning and tone. Output only the corrected text.',
                };
                setCtx({ ...ctx, post_processing: { ...pp, enabled: e.target.checked } });
              }}
              className="rounded mt-0.5"
            />
            <label htmlFor="ctx-pp-enabled" className="text-xs text-content-secondary cursor-pointer">
              Enable post-processing for this context
            </label>
          </div>

          {ctx.post_processing?.enabled && (
            <>
              <div>
                <label className="block text-xs font-medium mb-1">LLM Provider</label>
                <select
                  value={ctx.post_processing.llm_provider}
                  onChange={(e) => setCtx({
                    ...ctx,
                    post_processing: { ...ctx.post_processing!, llm_provider: e.target.value as ContextPostProcessing['llm_provider'] },
                  })}
                  className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                >
                  <option value="groq-llm">Groq (Llama - Fast)</option>
                  <option value="open-ai">OpenAI (GPT-4o-mini)</option>
                  <option value="anthropic">Anthropic (Claude Haiku)</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium mb-1">System Prompt</label>
                <textarea
                  value={ctx.post_processing.system_prompt}
                  onChange={(e) => setCtx({
                    ...ctx,
                    post_processing: { ...ctx.post_processing!, system_prompt: e.target.value },
                  })}
                  rows={4}
                  placeholder="Instructions for the AI on how to process the transcription..."
                  className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm resize-none"
                />
              </div>
            </>
          )}
        </div>
      </div>

      <button
        onClick={handleSave}
        disabled={saving || !ctx.name.trim()}
        className="flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-indigo-500 to-violet-500 text-white text-sm font-medium rounded-lg hover:from-indigo-600 hover:to-violet-600 disabled:opacity-50 transition-all"
      >
        <Save className="w-4 h-4" />
        {saving ? 'Saving...' : 'Save Context'}
      </button>
    </div>
  );
}
