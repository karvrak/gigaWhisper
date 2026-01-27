# Plan de Corrections Pré-Production - GigaWhisper

> Document généré le 2026-01-26
> **Total : 41 problèmes** - 5 critiques, 20 majeurs, 16 mineurs

---

## Phase 1 : CRITIQUES (Bloquants pour la production)

### [CRIT-01] Signature de code Windows
- **Priorité** : 1/5
- **Agent** : @architect puis @developer
- **Fichier** : `src-tauri/tauri.conf.json:79`
- **Problème** : `certificateThumbprint: null` - L'application n'est pas signée numériquement
- **Impact** : Windows SmartScreen bloquera l'installation, perte de confiance utilisateur
- **Action** :
  1. Obtenir un certificat de signature de code EV (Extended Validation)
  2. Configurer le thumbprint dans `tauri.conf.json`
  3. Mettre à jour le CI pour signer automatiquement
- **Référence** : `docs/ADR/ADR-005-windows-code-signing.md`

---

### [CRIT-02] Panics en production - expect() dans lib.rs
- **Priorité** : 2/5
- **Agent** : @developer
- **Fichiers** :
  - `src-tauri/src/lib.rs:76` - `expect("Failed to create log file appender")`
  - `src-tauri/src/lib.rs:224` - `expect("error while running tauri application")`
- **Problème** : Ces `expect()` causeront un crash sans message d'erreur clair
- **Impact** : Crash de l'application sans recovery possible
- **Action** :
  ```rust
  // Avant
  .expect("Failed to create log file appender")

  // Après
  .map_err(|e| {
      eprintln!("Failed to create log file appender: {}", e);
      // Fallback vers stdout ou désactiver les logs fichier
  })?
  ```
- **Tests requis** : Tester le démarrage avec dossier logs non accessible

---

### [CRIT-03] Panic dans RingBuffer
- **Priorité** : 3/5
- **Agent** : @developer
- **Fichier** : `src-tauri/src/audio/buffer.rs:22`
- **Problème** : `Self::try_new(capacity).expect(...)` paniquera si capacity = 0
- **Impact** : Crash si configuration invalide
- **Action** :
  ```rust
  // Ajouter validation en amont dans config/settings.rs
  pub fn validate(&self) -> Result<(), ConfigError> {
      if self.buffer_seconds <= 0.0 {
          return Err(ConfigError::InvalidValue("buffer_seconds must be > 0"));
      }
      Ok(())
  }
  ```
- **Tests requis** : Test avec capacity=0, test avec valeurs négatives

---

### [CRIT-04] Timeout manquant sur transcription locale
- **Priorité** : 4/5
- **Agent** : @developer
- **Fichier** : `src-tauri/src/transcription/whisper.rs`
- **Problème** : La transcription Whisper locale n'a pas de timeout
- **Impact** : Application freezée sans possibilité de recovery avec modèle corrompu
- **Action** :
  1. Ajouter un `CancellationToken` au contexte de transcription
  2. Implémenter un timeout configurable (défaut: 5 minutes)
  3. Émettre un événement d'erreur si timeout atteint
  ```rust
  use tokio::time::timeout;

  let result = timeout(
      Duration::from_secs(settings.transcription_timeout_secs),
      self.transcribe_internal(audio_data)
  ).await.map_err(|_| TranscriptionError::Timeout)?;
  ```
- **Tests requis** : Test avec audio très long, test d'annulation

---

### [CRIT-05] Absence de tests d'intégration
- **Priorité** : 5/5
- **Agent** : @tester
- **Fichiers** : Créer `src-tauri/tests/integration/`
- **Problème** : Aucun test end-to-end (enregistrement → transcription → collage)
- **Impact** : Bugs d'intégration non détectés avant release
- **Action** :
  1. Créer un mock pour l'interface audio (`MockAudioCapture`)
  2. Créer un mock pour l'API Groq (`MockGroqClient`)
  3. Tester le workflow complet :
     - Start recording → Stop → Transcribe → Paste
  4. Ajouter au CI
- **Structure suggérée** :
  ```
  src-tauri/tests/
  ├── integration/
  │   ├── mod.rs
  │   ├── recording_flow_test.rs
  │   ├── transcription_test.rs
  │   └── mocks/
  │       ├── audio.rs
  │       └── groq.rs
  ```

---

## Phase 2 : MAJEURS - Sécurité

### [MAJ-S01] CSP trop permissive
- **Priorité** : 6/20
- **Agent** : @developer
- **Fichier** : `src-tauri/tauri.conf.json:68`
- **Problème** : `'unsafe-inline'` dans style-src permet l'injection de styles
- **Action** :
  1. Générer des hashes pour les styles inline existants
  2. Remplacer `'unsafe-inline'` par les hashes spécifiques
  3. Ou migrer vers des classes CSS externes
- **CSP recommandée** :
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'sha256-xxx'; img-src 'self' data:; connect-src 'self' https://api.groq.com"
  ```

---

### [MAJ-S02] Clé API potentiellement exposée dans les logs
- **Priorité** : 7/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/transcription/groq.rs:132`
- **Problème** : En cas d'erreur réseau, le message pourrait contenir des credentials
- **Action** :
  ```rust
  // Ajouter un filtre pour masquer les credentials
  fn sanitize_error(error: &str) -> String {
      // Masquer les tokens Bearer
      let re = Regex::new(r"Bearer [A-Za-z0-9_-]+").unwrap();
      re.replace_all(error, "Bearer [REDACTED]").to_string()
  }
  ```

---

### [MAJ-S03] Validation des shortcuts système
- **Priorité** : 8/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/shortcuts/handler.rs:359-367`
- **Problème** : L'utilisateur peut configurer des shortcuts système dangereux
- **Action** :
  ```rust
  const BLACKLISTED_SHORTCUTS: &[&str] = &[
      "Alt+F4",
      "Ctrl+Alt+Delete",
      "Ctrl+Shift+Escape",
      "Alt+Tab",
      "Win+L",
  ];

  pub fn validate_shortcut(shortcut: &str) -> Result<(), ShortcutError> {
      if BLACKLISTED_SHORTCUTS.iter().any(|&s| s.eq_ignore_ascii_case(shortcut)) {
          return Err(ShortcutError::ReservedShortcut(shortcut.to_string()));
      }
      Ok(())
  }
  ```

---

## Phase 2 : MAJEURS - Robustesse

### [MAJ-R01] Race condition dans le downloader
- **Priorité** : 9/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/models/downloader.rs:159-177`
- **Problème** : Entre `is_downloading()` et `start_download()`, double téléchargement possible
- **Action** :
  ```rust
  // Utiliser un Mutex pour l'opération atomique
  pub async fn start_download_atomic(&self, model_id: &str) -> Result<(), DownloadError> {
      let _guard = self.download_lock.lock().await;
      if self.is_downloading(model_id) {
          return Err(DownloadError::AlreadyInProgress);
      }
      self.start_download_internal(model_id).await
  }
  ```

---

### [MAJ-R02] Fichiers temporaires non nettoyés
- **Priorité** : 10/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/models/downloader.rs:421-468`
- **Problème** : Fichier `.tmp` reste en cas d'erreur
- **Action** :
  ```rust
  // Utiliser un guard RAII
  struct TempFileGuard {
      path: PathBuf,
      keep: bool,
  }

  impl Drop for TempFileGuard {
      fn drop(&mut self) {
          if !self.keep {
              let _ = std::fs::remove_file(&self.path);
          }
      }
  }
  ```

---

### [MAJ-R03] Retry pour opérations fichier
- **Priorité** : 11/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/history/mod.rs:77-91`
- **Problème** : `save()` n'a pas de retry en cas d'échec
- **Action** :
  ```rust
  async fn save_with_retry(&self, max_retries: u32) -> Result<(), HistoryError> {
      let mut attempts = 0;
      loop {
          match self.save_internal().await {
              Ok(_) => return Ok(()),
              Err(e) if attempts < max_retries => {
                  attempts += 1;
                  tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempts))).await;
              }
              Err(e) => return Err(e),
          }
      }
  }
  ```

---

### [MAJ-R04] Gestion déconnexion microphone
- **Priorité** : 12/20
- **Agent** : @developer
- **Fichier** : `src-tauri/src/audio/capture.rs:196-211`
- **Problème** : Détection basée sur des strings - fragile
- **Action** :
  1. Utiliser les codes d'erreur cpal si disponibles
  2. Ajouter un heartbeat pour vérifier le device
  3. Émettre un événement `microphone-disconnected` pour notifier l'UI

---

## Phase 2 : MAJEURS - Tests

### [MAJ-T01] Tests d'intégration Tauri-React
- **Priorité** : 13/20
- **Agent** : @tester
- **Fichiers** : `src/tests/integration/`
- **Problème** : Pas de tests de la communication IPC
- **Action** :
  1. Installer `@tauri-apps/api` mock
  2. Créer des tests pour chaque commande Tauri
  3. Tester les événements bidirectionnels

---

### [MAJ-T02] Tests du module downloader
- **Priorité** : 14/20
- **Agent** : @tester
- **Fichier** : `src-tauri/src/models/downloader.rs`
- **Action** :
  1. Ajouter mockito pour simuler les erreurs HTTP
  2. Tester : timeout, 404, 500, connexion interrompue
  3. Tester la reprise de téléchargement

---

### [MAJ-T03] Tests du module updater
- **Priorité** : 15/20
- **Agent** : @tester
- **Fichier** : `src-tauri/src/updater.rs`
- **Action** :
  1. Créer des tests pour `check_for_updates`
  2. Tester les différents scénarios de version
  3. Mocker les réponses GitHub

---

## Phase 2 : MAJEURS - UX

### [MAJ-U01] Messages d'erreur techniques
- **Priorité** : 16/20
- **Agent** : @developer
- **Fichier** : `src/components/SettingsPanel.tsx:49`
- **Problème** : `setError(String(e))` affiche des erreurs brutes
- **Action** :
  ```typescript
  // Créer un mapper d'erreurs
  const ERROR_MESSAGES: Record<string, string> = {
    'ENOTFOUND': 'Impossible de se connecter au serveur. Vérifiez votre connexion internet.',
    'INVALID_API_KEY': 'Clé API invalide. Veuillez vérifier votre clé Groq.',
    'MODEL_NOT_FOUND': 'Modèle non trouvé. Veuillez télécharger le modèle.',
    // ...
  };

  function getUserFriendlyError(error: unknown): string {
    const errorStr = String(error);
    for (const [key, message] of Object.entries(ERROR_MESSAGES)) {
      if (errorStr.includes(key)) return message;
    }
    return 'Une erreur inattendue s\'est produite. Veuillez réessayer.';
  }
  ```

---

### [MAJ-U02] Feedback téléchargement modèle
- **Priorité** : 17/20
- **Agent** : @developer
- **Fichier** : `src/components/ModelSelector.tsx`
- **Action** :
  1. Ajouter une barre de progression avec pourcentage
  2. Afficher la taille téléchargée / taille totale
  3. Afficher l'estimation du temps restant
  4. Permettre l'annulation

---

## Phase 2 : MAJEURS - Configuration

### [MAJ-C01] Documentation variables CI
- **Priorité** : 18/20
- **Agent** : @developer
- **Fichier** : `.github/workflows/release.yml` et `README.md`
- **Action** :
  1. Documenter `TAURI_SIGNING_PRIVATE_KEY`
  2. Documenter `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  3. Ajouter une section "Configuration CI" dans CONTRIBUTING.md

---

### [MAJ-C02] Symboles de debug pour crash reporting
- **Priorité** : 19/20
- **Agent** : @developer
- **Fichier** : `src-tauri/Cargo.toml:123`
- **Problème** : `strip = true` supprime tous les symboles
- **Action** :
  ```toml
  [profile.release]
  strip = "debuginfo"  # Garder les symboles pour PDB
  split-debuginfo = "packed"  # Générer PDB séparé
  ```

---

## Phase 2 : MAJEURS - Documentation

### [MAJ-D01] Créer CHANGELOG.md
- **Priorité** : 20/20
- **Agent** : @developer
- **Fichier** : Créer `CHANGELOG.md`
- **Action** :
  ```markdown
  # Changelog

  All notable changes to this project will be documented in this file.

  The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

  ## [1.0.2] - 2026-01-XX

  ### Fixed
  - Bundle CUDA DLLs with installer
  - Clear corrupted cache and sync version

  ## [1.0.1] - 2026-01-XX

  ### Added
  - Initial release
  ```

---

### [MAJ-D02] Documentation API Tauri
- **Priorité** : 21/20
- **Agent** : @developer
- **Fichier** : Créer `docs/API.md`
- **Action** : Documenter toutes les commandes Tauri avec paramètres et retours

---

## Phase 2 : MAJEURS - Dépendances

### [MAJ-DEP01] Configurer cargo audit
- **Priorité** : 22/20
- **Agent** : @developer
- **Fichier** : `.github/workflows/release.yml`
- **Action** :
  ```yaml
  - name: Security audit
    run: |
      cargo install cargo-audit
      cargo audit
  ```

---

### [MAJ-DEP02] Vérifier whisper-rs
- **Priorité** : 23/20
- **Agent** : @researcher
- **Fichier** : `src-tauri/Cargo.toml:41`
- **Action** :
  1. Vérifier la stabilité de whisper-rs 0.14
  2. Surveiller les releases pour bugs critiques
  3. Considérer pinning à une version spécifique

---

## Phase 3 : MINEURS

### Qualité de code
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-Q01 | Magic numbers | `output/keyboard.rs:37,50` | Créer constantes `VK_CONTROL`, `VK_V` |
| MIN-Q02 | Variables non utilisées | Tests divers | Vérifier assertions |

### Robustesse
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-R01 | Buffer sous-dimensionné | `audio/capture.rs:138` | Avertir quand buffer presque plein |
| MIN-R02 | Validation sample_rate | `audio/format.rs` | Valider plage 8000-192000 |

### UX
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-U01 | Accessibilité aria-live | `SettingsPanel.tsx:226` | Ajouter `aria-live="polite"` |
| MIN-U02 | Confirmation suppression | `HistoryPanel.tsx` | Dialog de confirmation |
| MIN-U03 | Flash de thème | `App.tsx:46-50` | Lire thème depuis localStorage avant render |

### Configuration
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-C01 | Versions dupliquées | `tauri.conf.json`, `Cargo.toml`, `package.json` | Script de sync |
| MIN-C02 | RUSTFLAGS documentation | `release.yml:14` | Documenter le choix x86-64-v2 |

### Documentation
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-D01 | Troubleshooting manquant | `README.md` | Ajouter section FAQ |
| MIN-D02 | ADRs incomplets | `docs/ADR/` | Documenter whisper.cpp, cpal |

### Dépendances
| ID | Problème | Fichier | Action |
|----|----------|---------|--------|
| MIN-DEP01 | cpal features | `Cargo.toml:34` | Spécifier uniquement wasapi |
| MIN-DEP02 | webrtc-vad version | `Cargo.toml:38` | Vérifier mises à jour |
| MIN-DEP03 | Licences non vérifiées | CI | Ajouter cargo-deny |

---

## Checklist de Validation

Avant de passer en production, vérifier :

- [ ] Tous les CRITIQUES résolus
- [ ] Tous les MAJEURS Sécurité résolus
- [ ] Au moins 80% des MAJEURS résolus
- [ ] Tests d'intégration passent
- [ ] cargo audit sans vulnérabilités HIGH/CRITICAL
- [ ] Application signée et testée sur Windows 10/11
- [ ] CHANGELOG à jour
- [ ] Version synchronisée dans tous les fichiers

---

## Commandes Utiles

```bash
# Audit de sécurité
cargo audit

# Vérifier les licences
cargo deny check licenses

# Lancer les tests
cargo test --workspace

# Build release
cargo tauri build

# Vérifier les dépendances obsolètes
cargo outdated
```

---

## Notes pour les Agents

- **@developer** : Responsable des corrections de code
- **@tester** : Responsable des tests manquants
- **@reviewer** : Validation après chaque correction majeure
- **@architect** : Décisions sur signature de code et architecture

Chaque correction doit :
1. Être accompagnée de tests
2. Passer la review
3. Être documentée si nécessaire
