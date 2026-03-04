import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useCredits() {
  const [balance, setBalance] = useState<number>(0);
  const [isLow, setIsLow] = useState(false);
  const [isDepleted, setIsDepleted] = useState(false);

  useEffect(() => {
    // Initial load
    invoke<number>('get_credits_balance')
      .then(setBalance)
      .catch(console.error);

    const unsubscribes = [
      listen<{ balance_eur: number }>('credits:updated', (event) => {
        setBalance(event.payload.balance_eur);
        setIsDepleted(event.payload.balance_eur <= 0);
      }),
      listen<{ balance_eur: number }>('credits:low', (event) => {
        setIsLow(true);
        setBalance(event.payload.balance_eur);
      }),
      listen('credits:depleted', () => {
        setIsDepleted(true);
        setBalance(0);
      }),
    ];

    return () => {
      unsubscribes.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  return { balance, isLow, isDepleted };
}
