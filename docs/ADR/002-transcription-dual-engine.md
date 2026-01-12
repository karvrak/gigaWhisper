# ADR-002: Architecture Dual-Engine pour la Transcription

## Status
Accepted

## Context

GigaWhisper doit transcrire l'audio vocal en texte. Deux approches existent :

1. **Locale** : whisper.cpp execute sur la machine utilisateur
2. **Cloud** : API externe (Groq, OpenAI Whisper API, etc.)

Les utilisateurs ont des besoins varies :
- Certains privilegient la **confidentialite** (tout en local)
- D'autres veulent la **qualite maximale** sans GPU puissant
- Le **temps de reponse** est critique pour l'experience utilisateur

## Decision

Implementer une architecture **dual-engine** avec :
1. **whisper.cpp** (local) comme moteur par defaut
2. **Groq API** (cloud) comme option haute-performance

L'utilisateur peut choisir son provider dans les settings. Un fallback automatique est possible si le provider principal echoue.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              TranscriptionOrchestrator                   │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │           trait TranscriptionProvider            │    │
│  │  + transcribe(audio: &[f32]) -> Result<String>  │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│            ┌─────────────┴─────────────┐                │
│            ▼                           ▼                │
│  ┌─────────────────┐        ┌─────────────────┐        │
│  │ WhisperProvider │        │   GroqProvider  │        │
│  │                 │        │                 │        │
│  │ - whisper.cpp   │        │ - REST API      │        │
│  │ - Models local  │        │ - API Key       │        │
│  │ - CPU/GPU       │        │ - Rate limits   │        │
│  └─────────────────┘        └─────────────────┘        │
└─────────────────────────────────────────────────────────┘
```

## Consequences

### Positives
- **Flexibilite** : Utilisateur choisit selon ses priorites
- **Resilience** : Fallback si un provider est indisponible
- **Extensibilite** : Facile d'ajouter d'autres providers (OpenAI, local LLM)
- **Offline capable** : Mode local fonctionne sans internet

### Negatives
- **Complexite** : Deux implementations a maintenir
- **Modeles locaux** : Telechargement initial (75MB - 1.5GB selon modele)
- **Configuration** : Plus d'options pour l'utilisateur

## Provider Details

### whisper.cpp (Local)
```rust
// Configuration exposee
struct WhisperConfig {
    model: WhisperModel,      // tiny, base, small, medium, large
    language: Option<String>, // auto-detect ou force
    translate: bool,          // traduire vers anglais
    threads: usize,           // parallelisme CPU
    gpu: bool,                // acceleration GPU si disponible
}
```

**Modeles supportes** :
| Modele | Taille | VRAM | Qualite |
|--------|--------|------|---------|
| tiny   | 75 MB  | ~1GB | Basique |
| base   | 142 MB | ~1GB | Correct |
| small  | 466 MB | ~2GB | Bon     |
| medium | 1.5 GB | ~5GB | Tres bon|
| large  | 2.9 GB | ~10GB| Excellent|

### Groq API (Cloud)
```rust
struct GroqConfig {
    api_key: String,
    model: String,           // whisper-large-v3
    response_format: String, // json, text, verbose_json
}
```

**Avantages Groq** :
- Latence ultra-faible (~0.5s pour 30s audio)
- Modele large-v3 sans GPU local
- 100 requetes/jour gratuit

## Alternatives Considered

### OpenAI Whisper API seul
- **Rejete car** : Cout ($0.006/minute), pas de mode offline
- **Avantage** : API stable, bien documentee

### whisper.cpp seul
- **Rejete car** : Qualite limitee sur CPU faibles, pas de GPU = lent
- **Avantage** : 100% offline, confidentialite

### Faster-whisper (Python)
- **Rejete car** : Necessite Python runtime, complexifie le packaging
- **Avantage** : Plus rapide que whisper.cpp sur certains setups
