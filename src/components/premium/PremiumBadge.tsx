import { Lock } from 'lucide-react';
import { usePremium } from '../../hooks/usePremium';
import type { PremiumFeature } from '../../types/premium';

interface PremiumBadgeProps {
  /** Check a specific feature */
  feature?: PremiumFeature;
  /** Directly control locked state (overrides feature check) */
  locked?: boolean;
  className?: string;
}

export function PremiumBadge({ feature, locked, className = '' }: PremiumBadgeProps) {
  const { isPremium, isFeatureAvailable } = usePremium();

  // Determine if badge should show
  const isLocked = locked !== undefined
    ? locked
    : feature
      ? !isFeatureAvailable(feature)
      : !isPremium;

  if (!isLocked) return null;

  return (
    <span
      className={`inline-flex items-center gap-1 px-1.5 py-0.5 text-xs font-medium rounded-full ${
        isPremium
          ? 'bg-indigo-100 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400'
          : 'bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400'
      } ${className}`}
      title={`Requires Premium${!isPremium ? ' license' : ''}`}
    >
      <Lock className="w-3 h-3" />
      PRO
    </span>
  );
}
