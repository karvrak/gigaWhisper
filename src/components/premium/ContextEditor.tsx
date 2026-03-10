import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HotkeyInput } from '../HotkeyInput';
import { Save, ArrowLeft, Sparkles, AlertTriangle, Search, X, RefreshCw } from 'lucide-react';
import type { TranscriptionContext, ContextPostProcessing } from '../../types/premium';

interface RunningApp {
  process_name: string;
  window_title: string;
}

interface ProviderAvailability {
  groq: boolean;
  openai: boolean;
  deepgram: boolean;
  custom: boolean;
}

interface LlmProviderAvailability {
  groq_llm: boolean;
  openai: boolean;
  anthropic: boolean;
  custom_llm: boolean;
}

interface ContextEditorProps {
  context: TranscriptionContext;
  onSave: (ctx: TranscriptionContext) => Promise<void>;
  onCancel: () => void;
  providerAvailability?: ProviderAvailability;
  llmProviderAvailability?: LlmProviderAvailability;
  onNavigateToProviders?: () => void;
}

export function ContextEditor({ context, onSave, onCancel, providerAvailability, llmProviderAvailability, onNavigateToProviders }: ContextEditorProps) {
  const pa = providerAvailability ?? { groq: true, openai: true, deepgram: true, custom: true };
  const llmPa = llmProviderAvailability ?? { groq_llm: false, openai: false, anthropic: false, custom_llm: false };
  const hasAnyLlmProvider = llmPa.groq_llm || llmPa.openai || llmPa.anthropic || llmPa.custom_llm;
  const [ctx, setCtx] = useState(context);
  const [saving, setSaving] = useState(false);

  // Running apps state
  const [runningApps, setRunningApps] = useState<RunningApp[]>([]);
  const [loadingApps, setLoadingApps] = useState(false);
  const [appSearchQuery, setAppSearchQuery] = useState('');
  const [showAppDropdown, setShowAppDropdown] = useState(false);

  const fetchRunningApps = async () => {
    setLoadingApps(true);
    try {
      const apps = await invoke<RunningApp[]>('get_running_apps');
      setRunningApps(apps);
    } catch (e) {
      console.error('Failed to fetch running apps:', e);
    } finally {
      setLoadingApps(false);
    }
  };

  useEffect(() => {
    fetchRunningApps();
  }, []);

  const filteredApps = runningApps.filter(app => {
    const query = appSearchQuery.toLowerCase();
    const alreadySelected = (ctx.app_patterns || []).some(
      p => p.toLowerCase() === app.process_name.toLowerCase()
    );
    if (alreadySelected) return false;
    if (!query) return true;
    return app.process_name.toLowerCase().includes(query)
      || app.window_title.toLowerCase().includes(query);
  });

  const addAppPattern = (processName: string) => {
    const current = ctx.app_patterns || [];
    if (!current.some(p => p.toLowerCase() === processName.toLowerCase())) {
      setCtx({ ...ctx, app_patterns: [...current, processName] });
    }
    setAppSearchQuery('');
  };

  const removeAppPattern = (processName: string) => {
    setCtx({
      ...ctx,
      app_patterns: (ctx.app_patterns || []).filter(
        p => p.toLowerCase() !== processName.toLowerCase()
      ),
    });
  };

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
            <option value="custom" disabled={!pa.custom}>
              Custom Endpoint{!pa.custom ? ' (No API key)' : ''}
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

        {/* Custom Vocabulary */}
        <div>
          <label className="block text-xs font-medium mb-1">Custom Vocabulary</label>
          <textarea
            value={ctx.custom_vocabulary || ''}
            onChange={(e) => setCtx({ ...ctx, custom_vocabulary: e.target.value || null })}
            rows={2}
            placeholder="Technical terms, names, jargon (comma-separated)&#10;e.g., Kubernetes, TypeScript, PostgreSQL"
            className="w-full px-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm resize-none"
          />
          <p className="text-xs text-content-tertiary mt-1">
            Helps Whisper recognize domain-specific words
          </p>
        </div>

        {/* App Auto-Switch */}
        <div>
          <label className="block text-xs font-medium mb-1">Auto-switch Apps</label>

          {/* Selected apps as tags */}
          {(ctx.app_patterns || []).length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-2">
              {(ctx.app_patterns || []).map((pattern) => (
                <span
                  key={pattern}
                  className="inline-flex items-center gap-1 px-2 py-1 bg-accent/10 text-accent border border-accent/20 rounded-md text-xs"
                >
                  {pattern}
                  <button
                    type="button"
                    onClick={() => removeAppPattern(pattern)}
                    className="hover:text-red-400 transition-colors"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </span>
              ))}
            </div>
          )}

          {/* Search input with dropdown */}
          <div className="relative">
            <div className="flex items-center gap-1">
              <div className="relative flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-content-tertiary" />
                <input
                  type="text"
                  value={appSearchQuery}
                  onChange={(e) => {
                    setAppSearchQuery(e.target.value);
                    setShowAppDropdown(true);
                  }}
                  onFocus={() => setShowAppDropdown(true)}
                  placeholder="Search running apps..."
                  className="w-full pl-8 pr-3 py-2 border border-edge rounded-md bg-surface-tertiary text-sm"
                />
              </div>
              <button
                type="button"
                onClick={fetchRunningApps}
                disabled={loadingApps}
                className="p-2 border border-edge rounded-md bg-surface-tertiary hover:bg-surface-secondary transition-colors"
                title="Refresh app list"
              >
                <RefreshCw className={`w-4 h-4 text-content-tertiary ${loadingApps ? 'animate-spin' : ''}`} />
              </button>
            </div>

            {showAppDropdown && (
              <>
                {/* Click-away overlay */}
                <div className="fixed inset-0 z-10" onClick={() => setShowAppDropdown(false)} />
                <div className="absolute z-20 mt-1 w-full max-h-48 overflow-y-auto bg-surface-secondary border border-edge rounded-md shadow-lg">
                  {loadingApps ? (
                    <div className="px-3 py-2 text-xs text-content-tertiary">Loading...</div>
                  ) : filteredApps.length === 0 ? (
                    <div className="px-3 py-2 text-xs text-content-tertiary">
                      {appSearchQuery ? 'No matching apps found' : 'No apps detected'}
                    </div>
                  ) : (
                    filteredApps.map((app) => (
                      <button
                        key={app.process_name + app.window_title}
                        type="button"
                        onClick={() => {
                          addAppPattern(app.process_name);
                          setShowAppDropdown(false);
                        }}
                        className="w-full text-left px-3 py-2 hover:bg-accent/10 transition-colors border-b border-edge last:border-b-0"
                      >
                        <div className="text-sm font-medium">{app.process_name}</div>
                        <div className="text-xs text-content-tertiary truncate">{app.window_title}</div>
                      </button>
                    ))
                  )}
                </div>
              </>
            )}
          </div>

          <p className="text-xs text-content-tertiary mt-1">
            Auto-activate this context when these apps are in focus
          </p>
        </div>

        {/* Post-Processing */}
        <div className="border border-edge rounded-lg p-3 space-y-3">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-violet-500" />
            <span className="text-xs font-medium">AI Post-Processing</span>
          </div>

          {!hasAnyLlmProvider ? (
            <div
              className="flex items-start gap-2 p-2 bg-amber-500/10 border border-amber-500/20 rounded-md cursor-pointer hover:bg-amber-500/15 transition-colors"
              onClick={onNavigateToProviders}
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
                      <option value="groq-llm" disabled={!llmPa.groq_llm}>
                        Groq (Llama - Fast){!llmPa.groq_llm ? ' (No API key)' : ''}
                      </option>
                      <option value="open-ai" disabled={!llmPa.openai}>
                        OpenAI (GPT-4o-mini){!llmPa.openai ? ' (No API key)' : ''}
                      </option>
                      <option value="anthropic" disabled={!llmPa.anthropic}>
                        Anthropic (Claude Haiku){!llmPa.anthropic ? ' (No API key)' : ''}
                      </option>
                      <option value="custom-llm" disabled={!llmPa.custom_llm}>
                        Custom Endpoint{!llmPa.custom_llm ? ' (No API key)' : ''}
                      </option>
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
