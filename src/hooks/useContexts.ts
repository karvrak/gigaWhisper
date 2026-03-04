import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptionContext } from '../types/premium';

export function useContexts() {
  const [contexts, setContexts] = useState<TranscriptionContext[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadContexts = useCallback(async () => {
    try {
      const data = await invoke<TranscriptionContext[]>('get_contexts');
      setContexts(data);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadContexts();
  }, [loadContexts]);

  const saveContext = useCallback(async (context: TranscriptionContext) => {
    try {
      await invoke('save_context', { context });
      await loadContexts();
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, [loadContexts]);

  const deleteContext = useCallback(async (contextId: string) => {
    try {
      await invoke('delete_context', { contextId });
      await loadContexts();
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, [loadContexts]);

  return {
    contexts,
    loading,
    error,
    saveContext,
    deleteContext,
    reload: loadContexts,
  };
}
