# ADR-004: Injection Clavier et Paste Automatique

## Status
Accepted

## Context

Apres transcription, GigaWhisper doit inserer le texte dans l'application active. Deux scenarios :

1. **Champ de texte actif** : Coller directement le texte
2. **Pas de curseur** : Afficher popup avec le texte

Contraintes Windows :
- Certaines applications bloquent le paste (jeux, terminals securises)
- Certains champs utilisent des controles custom (pas de clipboard standard)
- Latence doit etre imperceptible

## Decision

Implementer une strategie multi-niveaux :

1. **Methode principale** : Clipboard + simulation Ctrl+V
2. **Fallback** : `SendInput` pour injection caractere par caractere
3. **Dernier recours** : Popup overlay avec bouton copier

## Implementation

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct KeyboardInjector;

impl KeyboardInjector {
    /// Strategie principale : Clipboard + Ctrl+V
    pub fn paste_via_clipboard(text: &str) -> Result<()> {
        // 1. Sauvegarder clipboard actuel
        let previous = clipboard::get_text()?;

        // 2. Mettre le texte dans clipboard
        clipboard::set_text(text)?;

        // 3. Simuler Ctrl+V
        Self::send_ctrl_v()?;

        // 4. Restaurer clipboard apres delai
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = clipboard::set_text(&previous);
        });

        Ok(())
    }

    /// Fallback : Injection caractere par caractere
    pub fn type_text(text: &str) -> Result<()> {
        for c in text.chars() {
            Self::send_unicode_char(c)?;
            // Petit delai pour eviter perte de caracteres
            std::thread::sleep(Duration::from_micros(500));
        }
        Ok(())
    }

    fn send_ctrl_v() -> Result<()> {
        unsafe {
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            ..Default::default()
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_V,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            ..Default::default()
                        },
                    },
                },
                // Key up events...
            ];

            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    fn send_unicode_char(c: char) -> Result<()> {
        unsafe {
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wScan: c as u16,
                            dwFlags: KEYEVENTF_UNICODE,
                            ..Default::default()
                        },
                    },
                },
                // Key up...
            ];

            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }
}
```

### Detection Fenetre Active

```rust
pub struct FocusDetector;

impl FocusDetector {
    /// Verifie si une fenetre avec champ texte est active
    pub fn has_text_input() -> bool {
        unsafe {
            let hwnd = GetForegroundWindow();
            let focused = GetFocus();

            // Verifier si le controle focused accepte du texte
            // Heuristique : envoyer WM_GETDLGCODE et verifier DLGC_HASSETSEL
            let code = SendMessageW(focused, WM_GETDLGCODE, WPARAM(0), LPARAM(0));
            code.0 & DLGC_HASSETSEL as isize != 0
        }
    }

    /// Recupere le nom de l'application active
    pub fn get_active_app_name() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            // ... get process name from PID
        }
    }
}
```

## Flow Decision

```
                    ┌─────────────────┐
                    │ Transcription   │
                    │   Complete      │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │ has_text_input  │
                    │      ?          │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              │ YES                         │ NO
              ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │ paste_via_      │           │  Show Popup     │
    │ clipboard()     │           │  Overlay        │
    └────────┬────────┘           └─────────────────┘
             │
             ▼
    ┌─────────────────┐
    │   Success?      │
    └────────┬────────┘
             │
    ┌────────┴────────┐
    │ YES             │ NO
    ▼                 ▼
  [Done]        ┌─────────────────┐
                │  type_text()    │
                │  (fallback)     │
                └─────────────────┘
```

## Consequences

### Positives
- **Robuste** : Multiple strategies de fallback
- **Rapide** : Ctrl+V est quasi instantane
- **Compatible** : Fonctionne avec la majorite des apps Windows
- **Non-intrusif** : Restaure le clipboard original

### Negatives
- **Clipboard ecrase** : Temporairement remplace le contenu utilisateur
- **Race conditions** : Si utilisateur copie pendant le paste
- **Apps securisees** : Certaines apps bloquent SendInput (anti-cheat, etc.)

## Edge Cases Geres

1. **Applications UAC elevated** : Peut echouer si GigaWhisper n'est pas admin
2. **Jeux fullscreen** : Utiliser popup overlay
3. **Remote Desktop** : SendInput peut ne pas fonctionner, utiliser clipboard
4. **Emojis/Unicode** : `KEYEVENTF_UNICODE` supporte UTF-16

## Alternatives Considered

### UI Automation API
- **Rejete car** : Complexe, overhead important
- **Avantage** : Plus fiable pour trouver les champs texte

### Direct WM_CHAR messages
- **Rejete car** : Ne fonctionne pas avec tous les controles
- **Avantage** : Plus direct

### SetWindowText
- **Rejete car** : Remplace tout le contenu, pas d'insertion
- **Avantage** : Simple pour certains cas
