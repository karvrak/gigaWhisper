//! Keyboard Injection
//!
//! Simulate keyboard input using Windows SendInput API.

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Keyboard injection errors
#[derive(Debug, thiserror::Error)]
pub enum KeyboardError {
    #[error("Failed to send input")]
    SendFailed,

    #[error("Platform not supported")]
    Unsupported,
}

/// Simulate Ctrl+V keypress
#[cfg(windows)]
pub fn send_ctrl_v() -> Result<(), KeyboardError> {
    use std::mem::size_of;

    // SAFETY: SendInput is safe to call because:
    // - The INPUT array is properly initialized with valid keyboard input structures
    // - All virtual key codes (VK_CONTROL=0x11, VK_V=0x56) are valid Windows constants
    // - The array is stack-allocated with known size, passed by reference
    // - size_of::<INPUT>() correctly computes the structure size
    // - We check the return value to detect partial failures
    // - No memory allocation or handles require cleanup
    unsafe {
        let inputs = [
            // Ctrl down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0x11), // VK_CONTROL
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0x56), // VK_V
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0x56), // VK_V
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Ctrl up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0x11), // VK_CONTROL
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(KeyboardError::SendFailed);
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn send_ctrl_v() -> Result<(), KeyboardError> {
    Err(KeyboardError::Unsupported)
}

/// Type text character by character using Unicode input
#[cfg(windows)]
pub fn type_text(text: &str) -> Result<(), KeyboardError> {
    use std::mem::size_of;

    for c in text.chars() {
        // SAFETY: SendInput with Unicode characters is safe because:
        // - KEYEVENTF_UNICODE flag tells Windows to interpret wScan as a Unicode character
        // - Rust chars are valid Unicode scalar values, safe to cast to u16 for BMP chars
        // - The INPUT array is properly initialized and stack-allocated
        // - We check return value and propagate errors on failure
        // - Small sleep between characters prevents input buffer overflow
        unsafe {
            let inputs = [
                // Key down with Unicode
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: c as u16,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                // Key up
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: c as u16,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];

            let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
            if sent != inputs.len() as u32 {
                return Err(KeyboardError::SendFailed);
            }
        }

        // Small delay between characters
        std::thread::sleep(std::time::Duration::from_micros(500));
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn type_text(_text: &str) -> Result<(), KeyboardError> {
    Err(KeyboardError::Unsupported)
}
