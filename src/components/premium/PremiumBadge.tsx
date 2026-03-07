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
          ? 'bg-accent/10 text-accent'
          : 'bg-surface-tertiary text-content-secondary'
      } ${className}`}
      title={`Requires Premium${!isPremium ? ' license' : ''}`}
    >
      <Lock className="w-3 h-3" />
      PRO
    </span>
  );
}
