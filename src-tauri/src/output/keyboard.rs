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
