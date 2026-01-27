# Actions Hors-Code pour Production

> Ce document liste toutes les actions manuelles, configurations externes et démarches administratives nécessaires pour finaliser la mise en production de GigaWhisper.

---

## 1. GitHub - Secrets et Configuration

### 1.1 Codecov (Couverture de tests)

**Priorité : Haute**

1. **Créer un compte Codecov**
   - Aller sur https://codecov.io
   - Se connecter avec GitHub
   - Autoriser l'accès au repo `karvrak/gigaWhisper`

2. **Récupérer le token**
   - Dans Codecov : Settings → General → Repository Upload Token
   - Copier le `CODECOV_TOKEN`

3. **Ajouter le secret dans GitHub**
   - Aller dans le repo → Settings → Secrets and variables → Actions
   - Cliquer "New repository secret"
   - Nom : `CODECOV_TOKEN`
   - Valeur : coller le token

---

### 1.2 Signature de Code Windows (Code Signing)

**Priorité : Critique**

#### Option A : Certificat EV (Extended Validation) - Recommandé

1. **Acheter un certificat EV Code Signing**
   - Fournisseurs recommandés :
     - DigiCert (~$500/an)
     - Sectigo (~$400/an)
     - GlobalSign (~$450/an)
   - L'EV élimine immédiatement les warnings SmartScreen

2. **Processus d'obtention (~1-2 semaines)**
   - Créer une entité légale (si pas déjà fait)
   - Soumettre documents d'entreprise
   - Vérification téléphonique
   - Réception du token USB (YubiKey ou similaire)

3. **Configurer le CI**
   - Exporter le certificat en base64 :
     ```bash
     base64 -i certificate.pfx -o cert_base64.txt
     ```
   - Ajouter les secrets GitHub :
     - `TAURI_SIGNING_PRIVATE_KEY` : contenu du cert_base64.txt
     - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : mot de passe du .pfx
     - `CERTIFICATE_THUMBPRINT` : empreinte SHA1 du certificat

4. **Mettre à jour tauri.conf.json**
   - Modifier `certificateThumbprint` avec la vraie valeur
   - Configurer `timestampUrl` : `"http://timestamp.digicert.com"`

#### Option B : Certificat OV (Organization Validation) - Budget limité

- Moins cher (~$200/an)
- SmartScreen affichera des warnings pendant ~2 semaines
- Processus similaire mais moins de vérifications

#### Option C : Auto-signé (Développement uniquement)

- Gratuit mais SmartScreen bloquera toujours
- À utiliser uniquement pour les tests internes

---

### 1.3 Autres Secrets GitHub (Optionnels)

| Secret | Description | Où l'obtenir |
|--------|-------------|--------------|
| `GITHUB_TOKEN` | Auto-généré par GitHub Actions | Automatique |
| `GROQ_API_KEY` | Pour tests CI avec Groq (optionnel) | https://console.groq.com |

---

## 2. Fichiers à Créer Manuellement

### 2.1 SECURITY.md

Le contenu a été généré par l'agent mais les permissions d'écriture étaient bloquées.

**Action :**
1. Créer le fichier `D:\VIBE-CODING\gigaWhisper\SECURITY.md`
2. Copier le contenu depuis la sortie de l'agent (disponible dans les logs de tâche)
3. Ou demander à Claude de régénérer le contenu

---

### 2.2 Structure E2E Tests (Playwright)

**Fichiers à créer :**

```
gigaWhisper/
├── playwright.config.ts          # Config fournie par l'agent
├── e2e/
│   ├── README.md
│   ├── setup/
│   │   ├── global-setup.ts
│   │   └── global-teardown.ts
│   ├── fixtures/
│   │   └── tauri-fixture.ts
│   └── tests/
│       └── app.spec.ts
```

**Action :**
1. Créer les dossiers : `mkdir -p e2e/setup e2e/fixtures e2e/tests`
2. Copier les fichiers depuis la sortie de l'agent
3. Installer les dépendances : `pnpm add -D @playwright/test playwright`
4. Installer Chromium : `npx playwright install chromium`

---

### 2.3 CHANGELOG.md

**Action :**
1. Créer `CHANGELOG.md` à la racine
2. Contenu initial :

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.2] - 2026-01-26

### Added
- SHA256 checksum verification for model downloads
- Comprehensive test coverage (~65%+)
- CI coverage reporting with Codecov
- SECURITY.md documentation
- ADR for crash reporting (opt-in)

### Fixed
- Path traversal vulnerability in history commands
- Panic on NaN in audio normalization
- Update endpoint mismatch (CPU/CUDA variants)
- Thread synchronization using channels instead of sleep

### Changed
- Log levels now conditional (debug in dev, warn in production)
- Improved idle model unloading

## [1.0.1] - 2026-01-XX

### Fixed
- Bundle CUDA DLLs with installer
- Clear corrupted cache

## [1.0.0] - 2026-01-XX

### Added
- Initial release
- Local Whisper transcription
- Groq cloud transcription
- Push-to-talk and toggle modes
- System tray integration
- Auto-updates
```

---

## 3. Services Externes

### 3.1 Groq API (Déjà configuré)

- Console : https://console.groq.com
- Documentation : https://console.groq.com/docs
- Rate limits : 60 RPM gratuit, 600 RPM payant

### 3.2 GitHub Releases

Les releases sont automatiques via GitHub Actions. Vérifier que :
- Le workflow `release.yml` a les permissions nécessaires
- Les artifacts sont correctement uploadés (CPU + CUDA variants)

---

## 4. Tests Manuels Avant Release

### 4.1 Checklist Windows

- [ ] Installer sur Windows 10 (clean install)
- [ ] Installer sur Windows 11 (clean install)
- [ ] Vérifier que SmartScreen n'affiche pas de warning (après signature)
- [ ] Tester avec antivirus actif (Windows Defender)
- [ ] Tester les raccourcis globaux (Ctrl+Space)
- [ ] Tester le mode push-to-talk
- [ ] Tester le mode toggle
- [ ] Tester transcription locale (Whisper)
- [ ] Tester transcription cloud (Groq)
- [ ] Tester auto-update depuis ancienne version
- [ ] Vérifier les logs dans `%APPDATA%\GigaWhisper\logs\`

### 4.2 Checklist Fonctionnelle

- [ ] Onboarding complet (nouveau utilisateur)
- [ ] Téléchargement de modèle (Tiny, Small)
- [ ] Changement de provider (local ↔ cloud)
- [ ] Configuration raccourci personnalisé
- [ ] Historique des transcriptions
- [ ] Suppression d'entrée historique
- [ ] Clear all historique
- [ ] Export transcription (si implémenté)

---

## 5. Documentation à Rédiger

### 5.1 README.md - Vérifications

- [ ] Badges à jour (CI, Coverage, Version)
- [ ] Instructions d'installation claires
- [ ] Screenshots de l'application
- [ ] Lien vers SECURITY.md
- [ ] Lien vers CHANGELOG.md

### 5.2 CONTRIBUTING.md (Optionnel)

Si vous acceptez des contributions :
- Guide de setup dev
- Standards de code
- Process de PR
- Configuration des secrets CI pour les forks

---

## 6. Publication

### 6.1 Avant la Release

1. **Mettre à jour la version** dans :
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`

2. **Créer le tag Git** :
   ```bash
   git tag -a v1.0.2 -m "Release 1.0.2"
   git push origin v1.0.2
   ```

3. **Vérifier le workflow** :
   - GitHub Actions → release.yml doit se déclencher
   - Attendre la fin du build (~15-20 min)
   - Vérifier les artifacts (GigaWhisper_x.x.x_x64-setup.exe)

### 6.2 Après la Release

1. **Vérifier la release GitHub** :
   - Tous les binaires présents (CPU + CUDA)
   - Release notes à jour
   - `latest-cpu.json` et `latest-cuda.json` présents

2. **Tester l'auto-update** :
   - Installer l'ancienne version
   - Vérifier que la notification de mise à jour apparaît
   - Tester la mise à jour

3. **Annoncer** (optionnel) :
   - Twitter/X
   - Reddit (r/whisper, r/opensource)
   - Hacker News

---

## 7. Monitoring Post-Release

### 7.1 À Surveiller

- **GitHub Issues** : bugs rapportés par les utilisateurs
- **Codecov** : couverture de tests
- **cargo audit** : nouvelles vulnérabilités
- **Dépendances** : mises à jour de whisper-rs, tauri, etc.

### 7.2 Commandes Utiles

```bash
# Vérifier les vulnérabilités Rust
cargo audit

# Vérifier les dépendances obsolètes
cargo outdated

# Vérifier les licences
cargo deny check licenses

# Audit npm
pnpm audit
```

---

## Résumé des Actions Prioritaires

| # | Action | Priorité | Temps estimé |
|---|--------|----------|--------------|
| 1 | Ajouter CODECOV_TOKEN dans GitHub | Haute | 5 min |
| 2 | Créer SECURITY.md | Haute | 10 min |
| 3 | Créer CHANGELOG.md | Haute | 15 min |
| 4 | Créer structure e2e/ | Moyenne | 20 min |
| 5 | Obtenir certificat code signing | Critique | 1-2 semaines |
| 6 | Configurer secrets CI pour signing | Critique | 30 min |
| 7 | Tests manuels Windows | Haute | 1-2 heures |

---

*Document généré le 2026-01-26*
