# Guide : Système de Licence Pro - GigaWhisper

## Vue d'ensemble

```
Utilisateur achete sur gigawhisper.com
        |
        v
   Stripe Checkout
        |
        v
   Webhook Stripe --> Backend Next.js (gigaWhisper-FRONT)
        |                    |
        |            Genere cle de licence
        |            Stocke dans Supabase
        |            Envoie email (Resend)
        v
   Page de succes --> Affiche la cle
        |
        v
   Utilisateur copie la cle dans l'app desktop
        |
        v
   App Tauri --> POST /v1/license/validate
        |              |
        |        Serveur valide + enregistre machine
        |
        v
   Cle stockee dans Windows Credential Manager
   Premium active
```

---

## 1. Creation d'une cle de licence

### Format

```
GW-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX
```

- Prefixe : `GW-`
- 4 blocs de 8 caracteres alphanumeriques
- Charset sans ambiguite : `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (exclut 0/O/1/I/L)
- **La cle est opaque** : aucune donnee encodee dedans, toute la validation se fait en base de donnees

### Generation (cote backend)

La cle est generee aleatoirement lors du webhook Stripe `checkout.session.completed` :

```javascript
// Exemple simplifie
const CHARSET = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
function generateLicenseKey() {
  const blocks = [];
  for (let b = 0; b < 4; b++) {
    let block = '';
    for (let i = 0; i < 8; i++) {
      block += CHARSET[crypto.getRandomValues(new Uint8Array(1))[0] % CHARSET.length];
    }
    blocks.push(block);
  }
  return `GW-${blocks.join('-')}`;
}
// Resultat : GW-A3F8K2M1-R7B4N9P6-X1C5D8E2-Q4W7Y0Z3
```

---

## 2. Schema de la base de donnees (Supabase/PostgreSQL)

```sql
CREATE TABLE licenses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  license_key TEXT UNIQUE NOT NULL,
  tier TEXT NOT NULL,                    -- 'pro' ou 'lifetime'
  status TEXT NOT NULL DEFAULT 'active', -- 'active', 'expired', 'revoked'

  -- Stripe
  stripe_customer_id TEXT,
  stripe_subscription_id TEXT,           -- NULL pour lifetime
  stripe_checkout_session_id TEXT,

  -- Contact
  email TEXT NOT NULL,

  -- Dates
  created_at TIMESTAMPTZ DEFAULT now(),
  expires_at TIMESTAMPTZ,               -- NULL pour lifetime
  cancelled_at TIMESTAMPTZ,

  -- Activations (max 3 machines)
  max_activations INT DEFAULT 3,
  active_machines JSONB DEFAULT '[]'
  -- Format: [{"machine_id": "sha256...", "activated_at": "2026-03-04T..."}]
);
```

---

## 3. Les endpoints API a implementer (cote site Next.js)

### 3.1 Checkout Stripe

```
POST /api/stripe/checkout
Body: { plan: "pro-monthly" | "pro-annual" | "lifetime" }
Response: { url: "https://checkout.stripe.com/..." }
```

Cree une session Stripe Checkout et redirige l'utilisateur.

### 3.2 Webhook Stripe

```
POST /api/stripe/webhook
```

Gere 3 evenements :

| Evenement | Action |
|-----------|--------|
| `checkout.session.completed` | Genere la cle, INSERT dans Supabase, envoie l'email |
| `invoice.paid` | Met a jour `expires_at` (renouvellement abo) |
| `customer.subscription.deleted` | Passe `status = 'expired'` |

### 3.3 Recuperation de la cle apres achat

```
GET /api/license/retrieve?session_id=cs_xxx
Response: { license_key: "GW-...", email: "user@example.com" }
```

Appele par la page de succes apres le paiement pour afficher la cle a l'utilisateur.

### 3.4 Validation de licence (appele par l'app desktop)

```
POST /api/v1/license/validate
Body: { key: "GW-...", machine_id: "sha256_du_hostname_username" }
Response: { valid: true, expires_at: 1735689600, tier: "pro" }
```

Logique serveur :
1. Cherche la cle dans la table `licenses`
2. Verifie que `status = 'active'`
3. Verifie que `expires_at` n'est pas depasse (ou NULL pour lifetime)
4. Verifie le nombre d'activations < `max_activations`
5. Ajoute/met a jour la machine dans `active_machines`
6. Retourne le resultat

**Codes d'erreur :**
- `404` : Cle inconnue
- `403` : Licence expiree ou revoquee
- `429` : Trop de machines activees (limite 3)

### 3.5 Desactivation

```
POST /api/v1/license/deactivate
Body: { key: "GW-...", machine_id: "sha256..." }
Response: { success: true }
```

Retire la machine de `active_machines`, liberant un slot d'activation.

---

## 4. Cote application desktop (Tauri/Rust)

### Fichiers concernes

| Fichier | Role |
|---------|------|
| `src-tauri/src/licensing/license.rs` | LicenseManager : validation, stockage credential |
| `src-tauri/src/licensing/gate.rs` | Feature gating (quelles features sont premium) |
| `src-tauri/src/licensing/credits.rs` | Suivi des credits cloud API |
| `src-tauri/src/commands/premium.rs` | Commandes Tauri exposees au frontend |
| `src-tauri/src/config/settings.rs` | PremiumSettings dans la config |

### URL de base pour les appels API

L'app desktop appelle : `https://api.gigawhisper.com/v1/license/validate`

> **A configurer** : soit via une variable d'environnement a la compilation, soit en dur dans le code Rust.

### Flux d'activation

1. L'utilisateur colle sa cle dans Settings > Premium
2. L'app genere un `machine_id` = `SHA256(hostname + username)`
3. POST vers `/v1/license/validate` avec `{ key, machine_id }`
4. Si valide : cle stockee dans **Windows Credential Manager** (via le crate `keyring`)
   - Service : `gigawhisper`
   - Nom : `license_key`
5. Config mise a jour : `premium.is_premium = true`, `premium.expires_at = timestamp`

### Revalidation automatique

- Toutes les **24 heures**, l'app re-valide la cle aupres du serveur
- **Grace period** : si pas de reseau, la licence reste valide **7 jours** apres la derniere validation
- Si le serveur repond `valid: false` -> premium desactive

### Desactivation

- L'utilisateur clique "Desactiver" dans l'app
- POST vers `/v1/license/deactivate`
- Cle supprimee du Credential Manager
- `premium.is_premium = false`

---

## 5. Cote frontend React

### Fichiers concernes

| Fichier | Role |
|---------|------|
| `src/types/premium.ts` | Types TypeScript (PremiumStatus, etc.) |
| `src/hooks/usePremium.ts` | Hook d'activation/desactivation |
| `src/hooks/useCredits.ts` | Hook pour les credits |
| `src/components/premium/LicensePanel.tsx` | UI d'activation de la cle |
| `src/components/premium/PremiumBadge.tsx` | Badge "PRO" sur les features verrouilees |

### Commandes Tauri disponibles

Le frontend appelle ces commandes Tauri (definies dans `commands/premium.rs`) :

```typescript
// Activer une licence
await invoke('activate_license', { key: 'GW-...' });

// Desactiver
await invoke('deactivate_license');

// Verifier le statut
const status = await invoke('get_premium_status');
// -> { is_premium: boolean, expires_at: number | null, tier: string }

// Verifier une feature specifique
const available = await invoke('check_feature', { feature: 'LlmPostProcessing' });

// Solde des credits
const credits = await invoke('get_credits_balance');
```

---

## 6. Features premium (feature gating)

5 features verrouillees derriere le premium :

| Feature | Description |
|---------|-------------|
| `CustomThemes` | Themes personnalises (au-dela de Light/Dark/System) |
| `MultiContext` | Contextes multiples de transcription avec raccourcis |
| `LlmPostProcessing` | Post-traitement LLM des transcriptions |
| `OpenAiProvider` | Provider cloud OpenAI Whisper |
| `DeepgramProvider` | Provider cloud Deepgram Nova |

C'est un modele **tout-ou-rien** : si `is_premium = true`, toutes les features sont debloquees.

---

## 7. Variables d'environnement requises (cote site)

```env
# Stripe
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRICE_PRO_MONTHLY=price_...
STRIPE_PRICE_PRO_ANNUAL=price_...
STRIPE_PRICE_LIFETIME=price_...
NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY=pk_live_...

# Supabase
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_SERVICE_ROLE_KEY=eyJ...

# Email (Resend)
RESEND_API_KEY=re_...
```

---

## 8. Plans tarifaires

| Plan | Type | Prix |
|------|------|------|
| Pro Monthly | Abonnement mensuel | X EUR/mois |
| Pro Annual | Abonnement annuel | X EUR/an |
| Lifetime | Paiement unique | 99 EUR |

---

## 9. Checklist pour la mise en place

### Cote Stripe
- [ ] Creer les 3 produits/prix dans Stripe Dashboard
- [ ] Configurer le webhook vers `https://gigawhisper.com/api/stripe/webhook`
- [ ] Evenements a ecouter : `checkout.session.completed`, `invoice.paid`, `customer.subscription.deleted`

### Cote Supabase
- [ ] Creer la table `licenses` (voir schema section 2)
- [ ] Creer les index
- [ ] Configurer les Row Level Security policies

### Cote site Next.js (gigaWhisper-FRONT)
- [ ] Implementer les 5 endpoints API (sections 3.1 a 3.5)
- [ ] Page de succes `/purchase/success` qui affiche la cle
- [ ] Page pricing qui lance le checkout
- [ ] Configurer les variables d'environnement

### Cote app desktop
- [ ] Verifier que l'URL de base API est correcte dans `license.rs`
- [ ] Tester l'activation avec une cle de test
- [ ] Tester la revalidation automatique
- [ ] Tester le mode offline (grace period 7 jours)
- [ ] Tester la desactivation

---

## 10. Reference

Pour plus de details techniques (code complet des endpoints, templates email, etc.), voir :
- `docs/LICENSING-ARCHITECTURE.md` - Specification complete du systeme
- `docs/PRE_PRODUCTION_FIXES.md` - Checklist pre-production
