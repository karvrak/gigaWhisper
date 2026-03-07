import { useCredits } from '../../hooks/useCredits';
import { Coins, AlertTriangle } from 'lucide-react';

export function CreditsMeter() {
  const { balance, isLow, isDepleted } = useCredits();

  const percentage = Math.max(0, Math.min(100, (balance / 60) * 100));

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-2">
          <Coins className="w-4 h-4 text-content-tertiary" />
          <span className="font-medium">Cloud Credits</span>
        </div>
        <span className={`font-mono text-sm ${
          isDepleted ? 'text-red-500' : isLow ? 'text-amber-500' : 'text-content-secondary'
        }`}>
          {balance.toFixed(2)} EUR
        </span>
      </div>

      {/* Progress Bar */}
      <div className="h-2 bg-surface-tertiary rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${
            isDepleted
              ? 'bg-red-500'
              : isLow
              ? 'bg-amber-500'
              : 'bg-gradient-to-r from-indigo-500 to-violet-500'
          }`}
          style={{ width: `${percentage}%` }}
        />
      </div>

      {/* Warning */}
      {isLow && !isDepleted && (
        <div className="flex items-center gap-1.5 text-xs text-amber-600 dark:text-amber-400">
          <AlertTriangle className="w-3 h-3" />
          Low credits - cloud features may be limited
        </div>
      )}
      {isDepleted && (
        <div className="flex items-center gap-1.5 text-xs text-red-600 dark:text-red-400">
          <AlertTriangle className="w-3 h-3" />
          No credits remaining - cloud features unavailable
        </div>
      )}
    </div>
  );
}
