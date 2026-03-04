# GigaWhisper — Architecture du Système de Licensing

## Vue d'ensemble

```
+-------------------+       +------------------------+       +------------------+
|   gigaWhisper      |       |   gigaWhisper-FRONT     |       |     Stripe       |
|   (Desktop App)    |       |   (Next.js + API)       |       |                  |
|                    |       |                         |       |                  |
| LicenseManager ----+-----> | /api/v1/license/validate|       |                  |
|                    |       | /api/v1/license/deactivate      |                  |
|                    |       |                         |       |                  |
|                    |       | /api/stripe/checkout  <-+-------+ Checkout Session |
|                    |       | /api/stripe/webhook   <-+-------+ Webhook Events   |
|                    |       |                         |       |                  |
|                    |       |      Supabase (DB)      |       |                  |
+-------------------+       +------------------------+       +------------------+
```

**Stack :** Next.js API Routes + Supabase (PostgreSQL) + Stripe + Resend (emails)

Tout le backend est hébergé dans le même projet Next.js (gigaWhisper-FRONT). Le sous-domaine `api.gigawhisper.com` pointe vers le même projet Vercel.

---

## Flux complet de bout en bout

### Étape 1 — Achat

```
Utilisateur → gigawhisper.com/pricing
           → Clique "Get Pro" ou "Get Lifetime"
           → POST /api/stripe/checkout { plan: "pro-monthly" }
           → Redirigé vers Stripe Checkout
           → Paie avec sa carte
```

### Étape 2 — Génération de la clé

```
Stripe → POST /api/stripe/webhook (event: checkout.session.completed)
       → Génère clé : GW-A3F8K2M1-R7B4N9P6-X1C5D8E2-Q4W7Y0Z3
       → INSERT dans Supabase (table licenses)
       → Envoie email avec la clé (Resend)
```

### Étape 3 — Livraison

```
Stripe redirige → /purchase/success?session_id=cs_xxx
                → GET /api/license/retrieve?session_id=cs_xxx
                → Affiche la clé à l'écran + "Copiez et collez dans l'app"
                (+ email de backup déjà envoyé)
```

### Étape 4 — Activation dans le desktop

```
Utilisateur ouvre GigaWhisper desktop
           → Settings > Premium > Colle la clé
           → activate_license("GW-A3F8K2M1-...")
              → POST https://api.gigawhisper.com/v1/license/validate
                 { key: "GW-...", machine_id: "sha256..." }
              → Réponse : { valid: true, expires_at: 1743724800, tier: "pro" }
           → Clé stockée dans le credential manager Windows
           → config.premium.is_premium = true
```

### Étape 5 — Revalidation automatique

```
Toutes les 24h, le desktop app revalide silencieusement
           → POST /v1/license/validate (même endpoint)
           → Si hors-ligne : grace period de 7 jours
```

### Étape 6 — Renouvellement (abonnements)

```
Stripe → POST /api/stripe/webhook (event: invoice.paid)
       → UPDATE licenses SET expires_at = nouvelle_date
       → Le desktop app voit la nouvelle date à la prochaine revalidation
```

### Étape 7 — Annulation

```
Stripe → POST /api/stripe/webhook (event: customer.subscription.deleted)
       → UPDATE licenses SET status = 'expired'
       → À la prochaine revalidation, le desktop app reçoit valid: false
```

---

## Format des clés de licence

```
GW-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX
```

- Préfixe `GW-` pour identifier la provenance
- 4 blocs de 8 caractères alphanumériques majuscules
- Charset sans ambiguïté : `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (pas de 0/O/1/I/L)
- Total : 35 caractères
- **Clés opaques** — aucune information encodée dedans, toute la validation se fait en base

### Algorithme de génération

```typescript
// src/lib/license-keys.ts
import { randomBytes } from "crypto";

const CHARSET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

export function generateLicenseKey(): string {
  const bytes = randomBytes(32);
  const blocks: string[] = [];

  for (let b = 0; b < 4; b++) {
    let block = "";
    for (let i = 0; i < 8; i++) {
      block += CHARSET[bytes[b * 8 + i] % CHARSET.length];
    }
    blocks.push(block);
  }

  return `GW-${blocks.join("-")}`;
}
```

---

## Base de données (Supabase)

### Schema SQL

```sql
CREATE TABLE licenses (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  license_key   TEXT UNIQUE NOT NULL,
  tier          TEXT NOT NULL CHECK (tier IN ('pro', 'lifetime')),
  status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'expired', 'revoked')),

  -- Stripe
  stripe_customer_id         TEXT,
  stripe_subscription_id     TEXT,   -- NULL pour lifetime
  stripe_checkout_session_id TEXT,

  -- Contact
  email         TEXT NOT NULL,

  -- Dates
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at    TIMESTAMPTZ,          -- NULL pour lifetime (jamais d'expiration)
  cancelled_at  TIMESTAMPTZ,

  -- Activation
  max_activations  INT NOT NULL DEFAULT 3,
  active_machines  JSONB NOT NULL DEFAULT '[]'::jsonb
  -- Format: [{"machine_id": "sha256...", "activated_at": "2026-03-04T..."}]
);

CREATE INDEX idx_licenses_key ON licenses (license_key);
CREATE INDEX idx_licenses_stripe_sub ON licenses (stripe_subscription_id);
CREATE INDEX idx_licenses_email ON licenses (email);
```

### Points clés

| Champ | Rôle |
|-------|------|
| `max_activations = 3` | L'utilisateur peut activer sur 3 machines max |
| `active_machines` (JSONB) | Liste des machines activées avec leur ID et date |
| `expires_at = NULL` | Pour les licences Lifetime (pas d'expiration) |
| `stripe_subscription_id = NULL` | Pour les licences Lifetime (pas d'abonnement récurrent) |

---

## API Routes à implémenter

### Structure des fichiers

```
gigaWhisper-FRONT/src/
├── app/
│   ├── api/
│   │   ├── stripe/
│   │   │   ├── checkout/route.ts       # Création session Stripe
│   │   │   └── webhook/route.ts        # Webhook Stripe → génère la clé
│   │   ├── v1/
│   │   │   └── license/
│   │   │       ├── validate/route.ts   # Validation clé (appelé par desktop)
│   │   │       └── deactivate/route.ts # Désactivation machine
│   │   └── license/
│   │       └── retrieve/route.ts       # Récupérer clé après achat
│   ├── purchase/
│   │   └── success/
│   │       └── page.tsx                # Page de succès post-achat
│   └── ...
├── lib/
│   ├── license-keys.ts                 # Génération de clés
│   ├── email.ts                        # Envoi email via Resend
│   ├── stripe.ts                       # Client Stripe
│   └── supabase.ts                     # Client Supabase server-side
└── ...
```

### 1. POST `/api/stripe/checkout`

Crée une session Stripe Checkout et redirige l'utilisateur.

```typescript
// src/app/api/stripe/checkout/route.ts
import { NextRequest, NextResponse } from "next/server";
import Stripe from "stripe";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY!);

const PRICE_IDS: Record<string, string> = {
  "pro-monthly": process.env.STRIPE_PRICE_PRO_MONTHLY!,
  "pro-annual": process.env.STRIPE_PRICE_PRO_ANNUAL!,
  "lifetime": process.env.STRIPE_PRICE_LIFETIME!,
};

export async function POST(req: NextRequest) {
  const { plan } = await req.json();
  const priceId = PRICE_IDS[plan];

  if (!priceId) {
    return NextResponse.json({ error: "Invalid plan" }, { status: 400 });
  }

  const session = await stripe.checkout.sessions.create({
    mode: plan === "lifetime" ? "payment" : "subscription",
    payment_method_types: ["card"],
    line_items: [{ price: priceId, quantity: 1 }],
    success_url: `${process.env.NEXT_PUBLIC_BASE_URL}/purchase/success?session_id={CHECKOUT_SESSION_ID}`,
    cancel_url: `${process.env.NEXT_PUBLIC_BASE_URL}/pricing`,
    metadata: { plan },
  });

  return NextResponse.json({ url: session.url });
}
```

### 2. POST `/api/stripe/webhook`

Reçoit les événements Stripe. C'est le cœur du système.

```typescript
// src/app/api/stripe/webhook/route.ts
import { NextRequest, NextResponse } from "next/server";
import Stripe from "stripe";
import { createClient } from "@supabase/supabase-js";
import { generateLicenseKey } from "@/lib/license-keys";
import { sendLicenseEmail } from "@/lib/email";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY!);
const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_KEY!
);

export async function POST(req: NextRequest) {
  const body = await req.text();
  const signature = req.headers.get("stripe-signature")!;

  let event: Stripe.Event;
  try {
    event = stripe.webhooks.constructEvent(
      body, signature, process.env.STRIPE_WEBHOOK_SECRET!
    );
  } catch {
    return NextResponse.json({ error: "Invalid signature" }, { status: 400 });
  }

  switch (event.type) {
    case "checkout.session.completed": {
      const session = event.data.object as Stripe.Checkout.Session;
      const email = session.customer_details?.email;
      const plan = session.metadata?.plan;
      if (!email || !plan) break;

      const tier = plan === "lifetime" ? "lifetime" : "pro";
      const licenseKey = generateLicenseKey();

      let expiresAt: string | null = null;
      if (tier === "pro" && session.subscription) {
        const sub = await stripe.subscriptions.retrieve(session.subscription as string);
        expiresAt = new Date(sub.current_period_end * 1000).toISOString();
      }

      await supabase.from("licenses").insert({
        license_key: licenseKey,
        tier,
        email,
        stripe_customer_id: session.customer as string,
        stripe_subscription_id: (session.subscription as string) || null,
        stripe_checkout_session_id: session.id,
        expires_at: expiresAt,
      });

      await sendLicenseEmail(email, licenseKey, tier);
      break;
    }

    case "invoice.paid": {
      const invoice = event.data.object as Stripe.Invoice;
      if (!invoice.subscription) break;
      const sub = await stripe.subscriptions.retrieve(invoice.subscription as string);
      await supabase
        .from("licenses")
        .update({
          status: "active",
          expires_at: new Date(sub.current_period_end * 1000).toISOString(),
        })
        .eq("stripe_subscription_id", invoice.subscription);
      break;
    }

    case "customer.subscription.deleted": {
      const sub = event.data.object as Stripe.Subscription;
      await supabase
        .from("licenses")
        .update({ status: "expired", cancelled_at: new Date().toISOString() })
        .eq("stripe_subscription_id", sub.id);
      break;
    }
  }

  return NextResponse.json({ received: true });
}
```

### 3. POST `/api/v1/license/validate`

Appelé par le desktop app pour activer et revalider la licence.

```typescript
// src/app/api/v1/license/validate/route.ts
import { NextRequest, NextResponse } from "next/server";
import { createClient } from "@supabase/supabase-js";

const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_KEY!
);

export async function POST(req: NextRequest) {
  const { key, machine_id } = await req.json();

  if (!key || !machine_id) {
    return NextResponse.json({ valid: false }, { status: 400 });
  }

  const { data: license } = await supabase
    .from("licenses")
    .select("*")
    .eq("license_key", key)
    .single();

  if (!license || license.status === "revoked") {
    return NextResponse.json({ valid: false }, { status: 403 });
  }

  if (license.status === "expired") {
    return NextResponse.json({ valid: false }, { status: 403 });
  }

  // Vérifier expiration pour les abonnements
  if (license.expires_at && new Date(license.expires_at) < new Date()) {
    await supabase
      .from("licenses")
      .update({ status: "expired" })
      .eq("id", license.id);
    return NextResponse.json({ valid: false }, { status: 403 });
  }

  // Gérer les activations de machines
  const machines: Array<{ machine_id: string; activated_at: string }> =
    license.active_machines || [];
  const alreadyActivated = machines.some((m) => m.machine_id === machine_id);

  if (!alreadyActivated) {
    if (machines.length >= license.max_activations) {
      return NextResponse.json(
        { valid: false, error: "Activation limit reached" },
        { status: 429 }
      );
    }
    machines.push({ machine_id, activated_at: new Date().toISOString() });
    await supabase
      .from("licenses")
      .update({ active_machines: machines })
      .eq("id", license.id);
  }

  return NextResponse.json({
    valid: true,
    expires_at: license.expires_at
      ? Math.floor(new Date(license.expires_at).getTime() / 1000)
      : null,
    tier: license.tier,
  });
}
```

### 4. POST `/api/v1/license/deactivate`

Libère un siège machine lors de la désactivation.

```typescript
// src/app/api/v1/license/deactivate/route.ts
import { NextRequest, NextResponse } from "next/server";
import { createClient } from "@supabase/supabase-js";

const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_KEY!
);

export async function POST(req: NextRequest) {
  const { key, machine_id } = await req.json();

  const { data: license } = await supabase
    .from("licenses")
    .select("id, active_machines")
    .eq("license_key", key)
    .single();

  if (!license) {
    return NextResponse.json({ ok: true }); // idempotent
  }

  const machines = (license.active_machines || []).filter(
    (m: { machine_id: string }) => m.machine_id !== machine_id
  );

  await supabase
    .from("licenses")
    .update({ active_machines: machines })
    .eq("id", license.id);

  return NextResponse.json({ ok: true });
}
```

### 5. GET `/api/license/retrieve`

Récupère la clé après le paiement (page success).

```typescript
// src/app/api/license/retrieve/route.ts
import { NextRequest, NextResponse } from "next/server";
import Stripe from "stripe";
import { createClient } from "@supabase/supabase-js";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY!);
const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_KEY!
);

export async function GET(req: NextRequest) {
  const sessionId = req.nextUrl.searchParams.get("session_id");
  if (!sessionId) {
    return NextResponse.json({ error: "Missing session_id" }, { status: 400 });
  }

  const session = await stripe.checkout.sessions.retrieve(sessionId);
  if (session.payment_status !== "paid") {
    return NextResponse.json({ error: "Payment not completed" }, { status: 402 });
  }

  const { data: license } = await supabase
    .from("licenses")
    .select("license_key, tier")
    .eq("stripe_checkout_session_id", sessionId)
    .single();

  if (!license) {
    return NextResponse.json({ error: "License not found" }, { status: 404 });
  }

  return NextResponse.json({
    licenseKey: license.license_key,
    tier: license.tier,
  });
}
```

---

## Compatibilité avec le Desktop App

Le `LicenseManager` Rust existant (`src-tauri/src/licensing/license.rs`) est **déjà compatible** :

| Aspect | Desktop (Rust) | API (Next.js) |
|--------|---------------|---------------|
| URL | `https://api.gigawhisper.com/v1/license/validate` | `/api/v1/license/validate/route.ts` |
| Requête | `{ key, machine_id }` | Attend `{ key, machine_id }` |
| Réponse | `{ valid, expires_at, tier }` | Retourne `{ valid, expires_at, tier }` |
| Stockage clé | Credential manager Windows (keyring) | — |
| Revalidation | Toutes les 24h | Même endpoint |
| Grace period | 7 jours hors-ligne | — |

**Aucune modification côté Rust n'est nécessaire.**

---

## Produits Stripe à créer

Créer dans le [Dashboard Stripe](https://dashboard.stripe.com/products) :

| Produit | Prix | Mode |
|---------|------|------|
| GigaWhisper Pro Monthly | 4.99 EUR/mois | Récurrent (subscription) |
| GigaWhisper Pro Annual | 39 EUR/an | Récurrent (subscription) |
| GigaWhisper Lifetime | 99 EUR | Ponctuel (one-time) |

Récupérer les `price_id` de chaque prix et les mettre dans les variables d'environnement.

---

## Variables d'environnement

```env
# .env.local (gigaWhisper-FRONT)

# Stripe
STRIPE_SECRET_KEY=sk_live_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRICE_PRO_MONTHLY=price_...
STRIPE_PRICE_PRO_ANNUAL=price_...
STRIPE_PRICE_LIFETIME=price_...
NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY=pk_live_...

# Supabase
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_SERVICE_KEY=eyJ...

# Resend (emails)
RESEND_API_KEY=re_...

# App
NEXT_PUBLIC_BASE_URL=https://gigawhisper.com
```

---

## Dépendances à installer

```bash
cd gigaWhisper-FRONT
npm install stripe @stripe/stripe-js resend @supabase/supabase-js
```

---

## Email de livraison (Resend)

```typescript
// src/lib/email.ts
import { Resend } from "resend";

const resend = new Resend(process.env.RESEND_API_KEY);

export async function sendLicenseEmail(
  email: string,
  licenseKey: string,
  tier: string
) {
  await resend.emails.send({
    from: "GigaWhisper <noreply@gigawhisper.com>",
    to: email,
    subject: `Your GigaWhisper ${tier === "lifetime" ? "Lifetime" : "Pro"} License Key`,
    html: `
      <h2>Thank you for your purchase!</h2>
      <p>Here is your license key:</p>
      <pre style="background:#f4f4f4;padding:16px;font-size:18px;letter-spacing:1px;border-radius:8px;">
${licenseKey}
      </pre>
      <p>To activate:</p>
      <ol>
        <li>Open GigaWhisper</li>
        <li>Go to Settings → Premium</li>
        <li>Paste your license key</li>
        <li>Click "Activate"</li>
      </ol>
      <p>You can activate on up to 3 machines.</p>
    `,
  });
}
```

---

## Configuration du domaine API

Le desktop app appelle `https://api.gigawhisper.com`. Deux options :

### Option A — Sous-domaine Vercel (recommandé)

Dans les settings Vercel du projet gigaWhisper-FRONT, ajouter `api.gigawhisper.com` comme domaine. Les API routes seront accessibles directement.

### Option B — DNS CNAME

Ajouter un enregistrement CNAME `api.gigawhisper.com → cname.vercel-dns.com` puis configurer le domaine dans Vercel.

---

## Checklist de mise en place

- [ ] Créer le projet Supabase et exécuter le schema SQL
- [ ] Installer les dépendances npm
- [ ] Créer les 3 produits/prix dans Stripe Dashboard
- [ ] Configurer les variables d'environnement (.env.local)
- [ ] Implémenter les 5 API routes
- [ ] Implémenter `lib/license-keys.ts`, `lib/email.ts`, `lib/supabase.ts`
- [ ] Créer la page `/purchase/success`
- [ ] Connecter les boutons pricing → sessions Checkout
- [ ] Configurer le webhook Stripe (pointer vers `/api/stripe/webhook`)
- [ ] Configurer le domaine `api.gigawhisper.com` dans Vercel
- [ ] Tester le flux complet en mode Stripe test (`sk_test_...`)
- [ ] Configurer Resend (vérifier le domaine d'envoi)
- [ ] Passer en mode Stripe live (`sk_live_...`)

---

## Décisions architecturales

### Clés opaques (pas de JWT/HMAC)

Les clés sont des identifiants aléatoires validés exclusivement en base de données.

- **Avantage :** Simple, révocation instantanée, pas de secret dans le binaire
- **Inconvénient :** Nécessite une connexion pour la première activation (mitigé par la grace period de 7 jours)

### API dans le même projet Next.js

Pas de microservice séparé. Un seul projet, un seul déploiement.

- **Avantage :** Simple, partage de code, coût zéro
- **Inconvénient :** Couplage site/API (acceptable pour un indie app)
