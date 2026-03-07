import { useState } from 'react';
import { HotkeyInput } from '../HotkeyInput';
import { Save, ArrowLeft } from 'lucide-react';
import type { TranscriptionContext } from '../../types/premium';

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
