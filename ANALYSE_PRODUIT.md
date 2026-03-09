# Analyse complète de gigaWhisper

*Date : 2026-03-08*

---

## 1. Inventaire des fonctionnalités actuelles

| Catégorie | Features |
|-----------|----------|
| **Transcription** | 4 providers (whisper.cpp local, Groq, OpenAI, Deepgram), fallback automatique |
| **Audio** | VAD (WebRTC 4 niveaux), resampling, sélection micro, ring buffer |
| **GPU** | 3 variantes build (CPU, Vulkan, CUDA) |
| **Modes** | Push-to-talk, Toggle, annulation, durée max configurable |
| **Post-processing** | LLM via OpenAI/Anthropic/Groq avec prompt personnalisable (premium) |
| **Context Presets** | Multi-contextes avec raccourci, langue, provider, prompt par contexte (premium) |
| **Output** | Auto-paste (Ctrl+V), auto-capitalisation, auto-ponctuation, popup si pas de champ texte |
| **Premium** | License manager, credits tracking, feature gating, grace period offline 7j |
| **UI** | Themes (system/light/dark + custom premium), onboarding 5 étapes, system tray |
| **Historique** | 100 entrées max, audio WAV associé, playback |
| **Sécurité** | Secrets dans Windows Credential Manager, validation API keys |
| **Auto-update** | Variant-aware (CPU/Vulkan/CUDA), progression, restart |
| **Logging** | Dual console+fichier, rotation 7 jours |

---

## 2. Problèmes de robustesse à corriger

### Sévérité haute

- **Clone inutile du buffer audio** (`service.rs:363`) — ~19MB copiés pour rien si VAD désactivé
- **Race condition recording** — fenêtre entre `read()` et `write()` sur l'état d'enregistrement
- **Credits jamais synchronisés** — les `pending_deductions` s'accumulent sans flush serveur

### Sévérité moyenne

- **History en JSON brut** — réécriture complète à chaque ajout, SQLite serait plus robuste
- **Pas de debounce sur updateSettings** — chaque toggle = écriture disque immédiate
- **Machine ID fragile** — `SHA-256(hostname+username)` change si rename machine/user
- **Pas de validation audio avant envoi cloud** — audio silencieux/bruité gaspille des crédits
- **Provider LLM reconstruit à chaque appel** — devrait être caché
- **Timestamp maison** — réimplémentation manuelle au lieu de `chrono`/`time`

### Sévérité basse

- Code streaming implémenté mais non connecté (code mort)
- Feature gate trop simpliste (tout ou rien, pas par feature)
- Pas de feedback utilisateur si auto-paste échoue

---

## 3. Analyse concurrentielle

### Concurrents principaux

#### SuperWhisper (macOS + iOS)

- **Prix** : Gratuit (15 min) / Pro $8.49/mois, $84.99/an, $250 lifetime
- **Moteur** : whisper.cpp local + modes AI
- **Forces** : Modes contextuels intelligents (adaptation auto à l'app active), custom vocabulary, intégration outils coding AI (Cursor, Claude Code), iOS companion app, Super Mode avec context awareness via accessibility APIs

#### Wispr Flow (macOS + Windows + iOS)

- **Prix** : Gratuit (2000 mots/semaine) / Pro $15/mois ou $144/an
- **Moteur** : Cloud AI (pas local)
- **Forces** : 97.2% accuracy, Command Mode (highlight + voice edit), auto-editing (filler words, grammaire), 100+ langues, developer jargon recognition, tone matching, personal dictionary, streaming temps réel

#### MacWhisper (macOS)

- **Prix** : Gratuit (Tiny/Base/Small) / Pro $69 lifetime
- **Moteur** : whisper.cpp local
- **Forces** : Batch transcription de fichiers audio, system audio recording (Zoom/Teams), export SRT/VTT, speaker diarization, translation, AI prompts intégrés

#### Buzz (open-source, multi-platform)

- **Prix** : Gratuit (open-source)
- **Moteur** : Whisper, Whisper.cpp, Faster Whisper, HuggingFace, OpenAI API
- **Forces** : Multi-backend, export TXT/SRT/VTT, Vulkan GPU, voice track separation, support Linux, modèles MMS Meta AI (1000+ langues)

#### TypeWhisper (Windows)

- **Prix** : Open-source
- **Moteur** : Plugin-based (SherpaOnnx, OpenAI, Groq)
- **Forces** : Marketplace de plugins, per-app overrides (langue/modèle/mode par app), cloud LLM translation, webhook plugins

### Tableau comparatif

| Feature | gigaWhisper | SuperWhisper | Wispr Flow | MacWhisper | Buzz |
|---------|:-:|:-:|:-:|:-:|:-:|
| Windows | **oui** | non | oui | non | oui |
| Local/offline | **oui** | oui | non | oui | oui |
| Multi-provider cloud | **3** | 0 | 1 | 1 | 1 |
| VAD intégré | **oui** | non | non | non | non |
| GPU (Vulkan+CUDA) | **oui** | non | N/A | oui | oui |
| Streaming temps réel | non | non | **oui** | non | oui |
| Command Mode (voice edit) | non | **oui** | **oui** | non | non |
| Custom vocabulary | non | **oui** | **oui** | non | non |
| Import fichiers audio | non | non | non | **oui** | **oui** |
| Export SRT/VTT | non | non | non | **oui** | **oui** |
| Per-app context auto | non | **oui** | non | non | non |
| System audio capture | non | non | non | **oui** | non |
| Speaker diarization | non | non | non | **oui** | non |
| Open-source | **oui** | non | non | non | oui |
| Prix | Gratuit+Premium | $8.49/mois | $15/mois | $69 once | Gratuit |

### Avantages compétitifs de gigaWhisper

1. **Gratuit + open-source sur Windows** — positionnement unique
2. **3 providers cloud + fallback automatique** — aucun concurrent ne fait ça
3. **VAD intégré** — exclusif
4. **GPU Vulkan + CUDA** — rare pour une app open-source
5. **Post-processing multi-LLM** — 3 providers (OpenAI, Anthropic, Groq)

---

## 4. Features manquantes par priorité d'impact

| Priorité | Feature | Qui le fait | Impact business |
|:--------:|---------|-------------|-----------------|
| 1 | **Streaming temps réel** | Wispr Flow, Buzz | Attente forte, réduit la latence perçue |
| 2 | **Command Mode** (sélectionner du texte + "rends ça plus concis") | Wispr Flow, SuperWhisper | Très différenciateur, gros WOW factor |
| 3 | **Custom vocabulary** (noms propres, jargon technique) | SuperWhisper, Wispr Flow | Réduit les erreurs récurrentes |
| 4 | **Import fichiers audio/vidéo** | MacWhisper, Buzz | Ouvre le marché créateurs/podcasters |
| 5 | **Export SRT/VTT** | MacWhisper, Buzz | Complément naturel de l'import fichiers |
| 6 | **Per-app context automatique** | SuperWhisper, TypeWhisper | Les presets existent déjà, manque la détection auto |
| 7 | **System audio capture** (Zoom/Teams) | MacWhisper | Marché réunions/meetings |
| 8 | **Auto-suppression filler words** | Wispr Flow | Qualité perçue de la transcription |
| 9 | **Speaker diarization** | MacWhisper | Niche mais demandé |
| 10 | **Support macOS/Linux** | Tous sauf TypeWhisper | Tauri le permet, gros boost d'adoption |

---

## 5. Recommandations

### Quick wins (robustesse)

- Fixer le clone inutile du buffer audio
- Ajouter un debounce sur updateSettings
- Remplacer le timestamp maison par `chrono`/`time`
- Cacher le provider LLM (ne pas le recréer à chaque appel)

### Quick wins (features)

- **Per-app context auto** : upgrade naturel des context presets existants (détecter l'app active)
- **Custom vocabulary** : pré-processing du prompt Whisper avec un dictionnaire personnel
- **Auto-suppression filler words** : ajout au post-processing LLM existant

### Investissements stratégiques

- **Command Mode + streaming temps réel** : les deux features qui séparent le plus de Wispr Flow ($15/mois). Les implémenter = "les mêmes features que Wispr Flow, gratuit et open-source"
- **Import fichiers audio** : ouvre un nouveau segment de marché (créateurs, podcasters)
- **Support macOS** : double potentiellement la base utilisateurs
