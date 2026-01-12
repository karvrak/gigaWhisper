# ADR-001: Choix du Framework Desktop - Tauri v2

## Status
Accepted

## Context

GigaWhisper necessite un framework desktop pour Windows avec les exigences suivantes :
- Performance native (latence minimale pour la transcription vocale)
- Integration facile avec whisper.cpp (library C++)
- Raccourcis globaux fiables
- System tray natif
- Injection clavier (paste automatique)
- Bundle leger pour distribution open-source

Les options evaluees etaient :
1. **Tauri** (Rust + WebView)
2. **Electron** (Node.js + Chromium)
3. **Flutter Desktop** (Dart)
4. **.NET WPF/MAUI** (C#)

## Decision

Nous choisissons **Tauri v2** avec Rust comme backend et React/TypeScript pour l'UI.

## Consequences

### Positives
- **Performance** : Rust compile en code natif, pas de GC, overhead minimal
- **Bundle size** : ~5-10 MB vs ~150 MB pour Electron
- **Securite** : Rust memory safety, pas de vulnerabilites buffer overflow
- **Integration whisper.cpp** : FFI C/Rust direct via `whisper-rs`
- **Ecosystem mature** : Plugins officiels pour shortcuts, tray, clipboard
- **Open-source friendly** : MIT license, communaute active (80k+ stars)

### Negatives
- **Courbe d'apprentissage** : Rust plus complexe que JavaScript/C#
- **WebView variabilite** : Depend de WebView2 sur Windows (pre-installe W10/W11)
- **Debugging** : Outils moins matures qu'Electron DevTools

### Risques mitiges
- WebView2 : Pre-installe sur Windows 10/11, fallback installeur si absent
- Complexite Rust : Utilisation de crates matures, patterns idiomatiques

## Alternatives Considered

### Electron
- **Rejete car** : Bundle trop lourd (150MB+), consommation RAM excessive (100MB+ idle)
- **Avantage non retenu** : Ecosysteme JavaScript plus accessible

### Flutter Desktop
- **Rejete car** : Integration C++ complexe via FFI Dart, moins de support Windows natif
- **Avantage non retenu** : UI declarative moderne

### .NET WPF
- **Rejete car** : Moins adapte open-source, interop C++ verbeux
- **Avantage non retenu** : Integration Windows parfaite
