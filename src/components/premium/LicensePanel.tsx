import { useState } from 'react';
import { usePremium } from '../../hooks/usePremium';
import { Key, Check, AlertCircle, Shield, Loader2 } from 'lucide-react';

export function LicensePanel() {
  const { status, isPremium, activateLicense, deactivateLicense, error } = usePremium();
  const [licenseKey, setLicenseKey] = useState('');
  const [activating, setActivating] = useState(false);
  const [activationError, setActivationError] = useState<string | null>(null);

  const handleActivate = async () => {
    if (!licenseKey.trim()) return;
    setActivating(true);
    setActivationError(null);
    try {
      await activateLicense(licenseKey.trim());
      setLicenseKey('');
    } catch (err) {
      setActivationError(String(err));
    } finally {
      setActivating(false);
    }
  };

  const handleDeactivate = async () => {
    if (!confirm('Are you sure you want to deactivate your license?')) return;
    try {
      await deactivateLicense();
    } catch (err) {
      setActivationError(String(err));
    }
  };

  return (
    <div className="space-y-6">
      {/* Status Card */}
      <div className={`p-4 rounded-xl border ${
        isPremium
          ? 'bg-gradient-to-r from-indigo-50 to-violet-50 dark:from-indigo-900/20 dark:to-violet-900/20 border-accent/30'
          : 'bg-surface-primary border-edge'
      }`}>
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-lg ${
            isPremium
              ? 'bg-gradient-to-br from-indigo-500 to-violet-500 text-white'
              : 'bg-surface-tertiary text-content-secondary'
          }`}>
            <Shield className="w-5 h-5" />
          </div>
          <div className="flex-1">
            <h3 className="font-semibold text-sm">
              {isPremium ? 'Premium Active' : 'Free Plan'}
            </h3>
            <p className="text-xs text-content-secondary">
              {isPremium
                ? `Valid until ${status?.expires_at ? new Date(status.expires_at * 1000).toLocaleDateString() : 'lifetime'}`
                : 'Unlock premium features with a license key'}
            </p>
          </div>
          {isPremium && (
            <span className="px-2 py-0.5 text-xs font-semibold rounded-full bg-gradient-to-r from-indigo-500 to-violet-500 text-white">
              PRO
            </span>
          )}
        </div>
      </div>

      {/* Activation Form */}
      {!isPremium && (
        <div className="space-y-3">
          <label className="block text-sm font-medium">License Key</label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Key className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-content-tertiary" />
              <input
                type="text"
                value={licenseKey}
                onChange={(e) => setLicenseKey(e.target.value)}
                placeholder="XXXX-XXXX-XXXX-XXXX"
                className="w-full pl-9 pr-3 py-2 border border-edge rounded-lg bg-surface-tertiary text-sm focus:ring-2 focus:ring-accent focus:border-accent"
                onKeyDown={(e) => e.key === 'Enter' && handleActivate()}
              />
            </div>
            <button
              onClick={handleActivate}
              disabled={activating || !licenseKey.trim()}
              className="px-4 py-2 bg-gradient-to-r from-indigo-500 to-violet-500 text-white text-sm font-medium rounded-lg hover:from-indigo-600 hover:to-violet-600 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center gap-2"
            >
              {activating ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Check className="w-4 h-4" />
              )}
              Activate
            </button>
          </div>
        </div>
      )}

      {/* Deactivate Button */}
      {isPremium && (
        <button
          onClick={handleDeactivate}
          className="text-sm text-content-secondary hover:text-red-500 transition-colors"
        >
          Deactivate License
        </button>
      )}

      {/* Error Display */}
      {(activationError || error) && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          {activationError || error}
        </div>
      )}

      {/* Feature List */}
      {status && (
        <div className="space-y-2">
          <h4 className="text-sm font-medium">Premium Features</h4>
          <div className="space-y-1">
            {status.features.map((feature) => (
              <div key={feature.id} className="flex items-center gap-2 text-sm">
                <div className={`w-4 h-4 rounded-full flex items-center justify-center ${
                  feature.available
                    ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400'
                    : 'bg-surface-tertiary text-content-muted'
                }`}>
                  {feature.available ? (
                    <Check className="w-3 h-3" />
                  ) : (
                    <span className="block w-1.5 h-0.5 bg-current rounded" />
                  )}
                </div>
                <span className={feature.available ? '' : 'text-content-muted'}>
                  {feature.name}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
