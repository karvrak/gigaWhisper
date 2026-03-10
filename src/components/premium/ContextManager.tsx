import { useState } from 'react';
import { useContexts } from '../../hooks/useContexts';
import { usePremium } from '../../hooks/usePremium';
import { useSettings } from '../../hooks/useSettings';
import { PremiumBadge } from './PremiumBadge';
import { ContextEditor } from './ContextEditor';
import { Plus, Trash2, ArrowLeft } from 'lucide-react';
import type { TranscriptionContext } from '../../types/premium';
import { CONTEXT_PRESETS } from '../../types/contextPresets';

export function ContextManager() {
  const { contexts, saveContext, deleteContext } = useContexts();
  const { isFeatureAvailable } = usePremium();
  const { settings } = useSettings();
  const providerAvailability = settings ? {
    groq: settings.transcription.groq.api_key_configured,
    openai: settings.transcription.openai.api_key_configured,
    deepgram: settings.transcription.deepgram.api_key_configured,
    custom: settings.transcription.custom.api_key_configured,
  } : { groq: false, openai: false, deepgram: false, custom: false };
  const llmProviderAvailability = settings ? {
    groq_llm: settings.transcription.groq.api_key_configured,
    openai: settings.post_processing.openai.api_key_configured,
    anthropic: settings.post_processing.anthropic.api_key_configured,
    custom_llm: settings.post_processing.custom_llm.api_key_configured,
  } : { groq_llm: false, openai: false, anthropic: false, custom_llm: false };
  const canUseContexts = isFeatureAvailable('multi-context');
  const [editingContext, setEditingContext] = useState<TranscriptionContext | null>(null);
  const [showPresetPicker, setShowPresetPicker] = useState(false);

  const handleAdd = () => {
    if (!canUseContexts) return;
    setShowPresetPicker(true);
  };

  const handlePickPreset = (presetKey: string | null) => {
    const preset = presetKey ? CONTEXT_PRESETS.find((p) => p.key === presetKey) : null;
    const newCtx: TranscriptionContext = {
      id: crypto.randomUUID(),
      name: preset?.name ?? '',
      shortcut: '',
      language: preset?.language ?? 'auto',
      provider: preset?.provider ?? 'local',
      model: null,
      post_processing: preset?.postProcessing ?? null,
      color: preset?.color ?? null,
      icon: preset?.icon ?? null,
      custom_vocabulary: preset?.customVocabulary ?? null,
      app_patterns: preset?.appPatterns ?? [],
    };
    setShowPresetPicker(false);
    setEditingContext(newCtx);
  };

  const handleSave = async (ctx: TranscriptionContext) => {
    await saveContext(ctx);
    setEditingContext(null);
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this context?')) return;
    await deleteContext(id);
  };

  if (showPresetPicker) {
    return (
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <button onClick={() => setShowPresetPicker(false)} className="p-1 hover:bg-surface-tertiary rounded">
            <ArrowLeft className="w-4 h-4" />
          </button>
          <h3 className="text-sm font-medium">New Context</h3>
        </div>

        <p className="text-xs text-content-secondary">
          Start from a preset or create a blank context.
        </p>

        <div className="space-y-2">
          {CONTEXT_PRESETS.map((preset) => (
            <button
              key={preset.key}
              onClick={() => handlePickPreset(preset.key)}
              className="w-full flex items-center gap-3 p-3 rounded-lg border border-edge hover:border-accent/50 hover:bg-accent/5 transition-colors text-left"
            >
              <div
                className="w-8 h-8 rounded-lg flex items-center justify-center text-lg shrink-0"
                style={{ backgroundColor: preset.color + '20' }}
              >
                {preset.icon}
              </div>
              <div className="min-w-0">
                <div className="font-medium text-sm">{preset.name}</div>
                <div className="text-xs text-content-secondary">{preset.description}</div>
              </div>
            </button>
          ))}

          <button
            onClick={() => handlePickPreset(null)}
            className="w-full flex items-center gap-3 p-3 rounded-lg border border-dashed border-edge hover:border-accent/50 hover:bg-accent/5 transition-colors text-left"
          >
            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-lg shrink-0 bg-surface-tertiary">
              <Plus className="w-4 h-4 text-content-tertiary" />
            </div>
            <div className="min-w-0">
              <div className="font-medium text-sm">Blank Context</div>
              <div className="text-xs text-content-secondary">Start from scratch</div>
            </div>
          </button>
        </div>
      </div>
    );
  }

  if (editingContext) {
    return (
      <ContextEditor
        context={editingContext}
        onSave={handleSave}
        onCancel={() => setEditingContext(null)}
        providerAvailability={providerAvailability}
        llmProviderAvailability={llmProviderAvailability}
        onNavigateToProviders={() => {
          // Navigate to transcription tab in settings
          const tabSelect = document.querySelector('select') as HTMLSelectElement;
          if (tabSelect) {
            tabSelect.value = 'transcription';
            tabSelect.dispatchEvent(new Event('change', { bubbles: true }));
          }
          setEditingContext(null);
        }}
      />
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">Transcription Contexts</h3>
          <PremiumBadge feature="multi-context" />
        </div>
        <button
          onClick={handleAdd}
          disabled={!canUseContexts}
          className="flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-accent border border-accent/30 rounded-lg hover:bg-accent/5 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          <Plus className="w-3 h-3" />
          Add Context
        </button>
      </div>

      <div className="space-y-2">
        {contexts.map((ctx) => (
          <div
            key={ctx.id}
            className="flex items-center gap-3 p-3 rounded-lg border border-edge hover:border-edge-hover transition-colors cursor-pointer"
            onClick={() => canUseContexts && setEditingContext(ctx)}
          >
            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-lg" style={{ backgroundColor: ctx.color || '#6366f1' + '20' }}>
              {ctx.icon || ctx.name.charAt(0)}
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium text-sm">{ctx.name || 'Unnamed'}</div>
              <div className="text-xs text-content-secondary">
                {ctx.shortcut || 'No shortcut'} - {ctx.language === 'auto' ? 'Auto-detect' : ctx.language} - {ctx.provider}
              </div>
            </div>
            {ctx.id !== 'default' && canUseContexts && (
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(ctx.id); }}
                className="p-1.5 text-content-tertiary hover:text-red-500 transition-colors"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
