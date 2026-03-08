import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { PremiumStatus, PremiumFeature } from '../types/premium';

export function usePremium() {
  const [status, setStatus] = useState<PremiumStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    try {
      const data = await invoke<PremiumStatus>('get_premium_status');
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // Listen for credit updates
  useEffect(() => {
    const unsubscribes = [
      listen('premium:changed', () => loadStatus()),
      listen('credits:updated', () => loadStatus()),
      listen('credits:low', () => loadStatus()),
      listen('credits:depleted', () => loadStatus()),
    ];

    return () => {
      unsubscribes.forEach((p) => p.then((fn) => fn()));
    };
  }, [loadStatus]);

  const activateLicense = useCallback(async (key: string) => {
    try {
      const data = await invoke<PremiumStatus>('activate_license', { key });
      setStatus(data);
      setError(null);
      return data;
    } catch (err) {
      const errorMsg = String(err);
      setError(errorMsg);
      throw new Error(errorMsg);
    }
  }, []);

  const deactivateLicense = useCallback(async () => {
    try {
      await invoke('deactivate_license');
      await loadStatus();
    } catch (err) {
      setError(String(err));
      throw err;
    }
  }, [loadStatus]);

  const isFeatureAvailable = useCallback(
    (feature: PremiumFeature): boolean => {
      if (!status) return false;
      const f = status.features.find((s) => s.id === feature);
      return f?.available ?? false;
    },
    [status]
  );

  return {
    status,
    loading,
    error,
    isPremium: status?.is_premium ?? false,
    activateLicense,
    deactivateLicense,
    isFeatureAvailable,
    creditsBalance: status?.credits_balance ?? 0,
    creditsLow: status?.credits_low ?? false,
  };
}
