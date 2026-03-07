import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Eye, EyeOff, Trash2, Check, Loader2 } from 'lucide-react';

interface ApiKeyInputProps {
  provider: string;
  label: string;
  placeholder: string;
  isConfigured: boolean;
  setCommand: string;
  clearCommand: string;
  validateCommand?: string;
  onStatusChange: () => void;
}

export function ApiKeyInput({
  provider,
  label,
  placeholder,
  isConfigured,
  setCommand,
  clearCommand,
  validateCommand,
  onStatusChange,
}: ApiKeyInputProps) {
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const handleSave = async () => {
    if (!apiKey.trim()) return;
    setSaving(true);
    setError(null);
    setSuccess(false);
    try {
      // Validate the key against the real API first
      if (validateCommand) {
        await invoke(validateCommand, { apiKey: apiKey.trim() });
      }
      // Key is valid — store it
      await invoke(setCommand, { apiKey: apiKey.trim() });
      setApiKey('');
      setSuccess(true);
      onStatusChange();
      setTimeout(() => setSuccess(false), 3000);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleClear = async () => {
    setSaving(true);
    setError(null);
    try {
      await invoke(clearCommand);
      onStatusChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (isConfigured) {
    return (
      <div className="space-y-1">
        <label className="block text-xs font-medium">{label}</label>
        <div className="flex items-center gap-2">
          <div className="flex-1 flex items-center gap-2 px-3 py-2 border border-green-300 dark:border-green-500/30 rounded-md bg-green-50 dark:bg-green-900/10 text-sm">
            <Check className="w-4 h-4 text-green-500" />
            <span className="text-green-700 dark:text-green-400">API key configured</span>
          </div>
          <button
            onClick={handleClear}
            disabled={saving}
            className="p-2 text-content-tertiary hover:text-red-500 transition-colors"
            title={`Remove ${provider} API key`}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      <label className="block text-xs font-medium">{label}</label>
      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <input
            type={showKey ? 'text' : 'password'}
            value={apiKey}
            onChange={(e) => { setApiKey(e.target.value); setError(null); setSuccess(false); }}
            placeholder={placeholder}
            className="w-full px-3 py-2 pr-9 border border-edge rounded-md bg-surface-tertiary text-sm"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-content-tertiary hover:text-content-secondary"
          >
            {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
          </button>
        </div>
        <button
          onClick={handleSave}
          disabled={saving || !apiKey.trim()}
          className="px-3 py-2 text-xs font-medium text-white bg-accent hover:bg-accent-hover rounded-md disabled:opacity-50 transition-colors flex items-center gap-1.5"
        >
          {saving ? (
            <>
              <Loader2 className="w-3 h-3 animate-spin" />
              Validating...
            </>
          ) : 'Save'}
        </button>
      </div>
      {error && <p className="text-xs text-red-500">{error}</p>}
      {success && <p className="text-xs text-green-500">API key validated and saved</p>}
    </div>
  );
}
