import { useState } from 'react';
import { useContexts } from '../../hooks/useContexts';
import { usePremium } from '../../hooks/usePremium';
import { PremiumBadge } from './PremiumBadge';
import { ContextEditor } from './ContextEditor';
import { Plus, Trash2 } from 'lucide-react';
import type { TranscriptionContext } from '../../types/premium';

export function ContextManager() {
  const { contexts, saveContext, deleteContext } = useContexts();
  const { isFeatureAvailable } = usePremium();
  const canUseContexts = isFeatureAvailable('multi-context');
  const [editingContext, setEditingContext] = useState<TranscriptionContext | null>(null);

  const handleAdd = () => {
    if (!canUseContexts) return;
    const newCtx: TranscriptionContext = {
      id: crypto.randomUUID(),
      name: '',
      shortcut: '',
      language: 'auto',
      provider: 'local',
      model: null,
      post_processing: null,
      color: null,
      icon: null,
    };
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

  if (editingContext) {
    return (
      <ContextEditor
        context={editingContext}
        onSave={handleSave}
        onCancel={() => setEditingContext(null)}
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
          className="flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-500/30 rounded-lg hover:bg-indigo-50 dark:hover:bg-indigo-500/10 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          <Plus className="w-3 h-3" />
          Add Context
        </button>
      </div>

      <div className="space-y-2">
        {contexts.map((ctx) => (
          <div
            key={ctx.id}
            className="flex items-center gap-3 p-3 rounded-lg border border-gray-200 dark:border-violet-500/10 hover:border-gray-300 dark:hover:border-violet-500/20 transition-colors cursor-pointer"
            onClick={() => canUseContexts && setEditingContext(ctx)}
          >
            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-lg" style={{ backgroundColor: ctx.color || '#6366f1' + '20' }}>
              {ctx.icon || ctx.name.charAt(0)}
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium text-sm">{ctx.name || 'Unnamed'}</div>
              <div className="text-xs text-gray-500 dark:text-gray-400">
                {ctx.shortcut || 'No shortcut'} - {ctx.language === 'auto' ? 'Auto-detect' : ctx.language} - {ctx.provider}
              </div>
            </div>
            {ctx.id !== 'default' && canUseContexts && (
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(ctx.id); }}
                className="p-1.5 text-gray-400 hover:text-red-500 transition-colors"
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
