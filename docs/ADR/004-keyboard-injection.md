# ADR-004: Keyboard Injection and Automatic Paste

## Status
Accepted

## Context

After transcription, GigaWhisper must insert the text into the active application. Two scenarios:

1. **Active text field**: Paste the text directly
2. **No cursor**: Display popup with the text

Windows constraints:
- Some applications block paste (games, secure terminals)
- Some fields use custom controls (no standard clipboard)
- Latency must be imperceptible

## Decision

Implement a multi-level strategy:

1. **Primary method**: Clipboard + Ctrl+V simulation
2. **Fallback**: `SendInput` for character-by-character injection
3. **Last resort**: Popup overlay with copy button

## Implementation

```rust
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct KeyboardInjector;

impl KeyboardInjector {
    /// Primary strategy: Clipboard + Ctrl+V
    pub fn paste_via_clipboard(text: &str) -> Result<()> {
        // 1. Save current clipboard
        let previous = clipboard::get_text()?;

        // 2. Put text in clipboard
        clipboard::set_text(text)?;

        // 3. Simulate Ctrl+V
        Self::send_ctrl_v()?;

        // 4. Restore clipboard after delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = clipboard::set_text(&previous);
        });

        Ok(())
    }

    /// Fallback: Character-by-character injection
    pub fn type_text(text: &str) -> Result<()> {
        for c in text.chars() {
            Self::send_unicode_char(c)?;
            // Small delay to avoid character loss
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

### Active Window Detection

```rust
pub struct FocusDetector;

impl FocusDetector {
    /// Check if a window with text field is active
    pub fn has_text_input() -> bool {
        unsafe {
            let hwnd = GetForegroundWindow();
            let focused = GetFocus();

            // Check if the focused control accepts text
            // Heuristic: send WM_GETDLGCODE and check DLGC_HASSETSEL
            let code = SendMessageW(focused, WM_GETDLGCODE, WPARAM(0), LPARAM(0));
            code.0 & DLGC_HASSETSEL as isize != 0
        }
    }

    /// Get the active application name
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

## Decision Flow

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
- **Robust**: Multiple fallback strategies
- **Fast**: Ctrl+V is nearly instantaneous
- **Compatible**: Works with most Windows apps
- **Non-intrusive**: Restores original clipboard

### Negatives
- **Clipboard overwritten**: Temporarily replaces user content
- **Race conditions**: If user copies during paste
- **Secure apps**: Some apps block SendInput (anti-cheat, etc.)

## Handled Edge Cases

1. **UAC elevated applications**: May fail if GigaWhisper is not admin
2. **Fullscreen games**: Use popup overlay
3. **Remote Desktop**: SendInput may not work, use clipboard
4. **Emojis/Unicode**: `KEYEVENTF_UNICODE` supports UTF-16

## Alternatives Considered

### UI Automation API
- **Rejected because**: Complex, significant overhead
- **Advantage**: More reliable for finding text fields

### Direct WM_CHAR messages
- **Rejected because**: Doesn't work with all controls
- **Advantage**: More direct

### SetWindowText
- **Rejected because**: Replaces all content, no insertion
- **Advantage**: Simple for certain cases
