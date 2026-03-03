//! Mouse Hook for Mouse Button Shortcuts
//!
//! Uses Windows low-level mouse hook (WH_MOUSE_LL) to capture
//! extra mouse buttons (Mouse4/Mouse5) as global shortcuts.
//! This is needed because tauri-plugin-global-shortcut (muda) only
//! supports keyboard shortcuts.

#[cfg(windows)]
use parking_lot::Mutex;
#[cfg(windows)]
use std::sync::OnceLock;
#[cfg(windows)]
use tauri::AppHandle;

#[cfg(windows)]
use windows::Win32::Foundation::*;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::*;

// Windows message constants for extra mouse buttons
#[cfg(windows)]
const WM_XBUTTONDOWN: u32 = 0x020B;
#[cfg(windows)]
const WM_XBUTTONUP: u32 = 0x020C;

#[cfg(windows)]
struct MouseHookConfig {
    /// 1 = XBUTTON1 (Mouse4), 2 = XBUTTON2 (Mouse5)
    target_button: u16,
    require_ctrl: bool,
    require_alt: bool,
    require_shift: bool,
    app_handle: AppHandle,
}

#[cfg(windows)]
struct HookState {
    config: Option<MouseHookConfig>,
    thread_id: Option<u32>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

// Safety: HHOOK is isize (Send+Sync), AppHandle is Send+Sync, JoinHandle<()> is Send
#[cfg(windows)]
unsafe impl Send for HookState {}

#[cfg(windows)]
static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();

#[cfg(windows)]
fn get_state() -> &'static Mutex<HookState> {
    STATE.get_or_init(|| {
        Mutex::new(HookState {
            config: None,
            thread_id: None,
            thread_handle: None,
        })
    })
}

/// Check if a shortcut string refers to a mouse button
pub fn is_mouse_shortcut(shortcut: &str) -> bool {
    shortcut.contains("Mouse4") || shortcut.contains("Mouse5")
}

/// Parse a mouse shortcut string into (button_id, ctrl, alt, shift)
#[cfg(windows)]
fn parse_mouse_shortcut(shortcut: &str) -> Option<(u16, bool, bool, bool)> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut button: Option<u16> = None;

    for part in &parts {
        match part.trim() {
            "Ctrl" => ctrl = true,
            "Alt" => alt = true,
            "Shift" => shift = true,
            "Mouse4" => button = Some(1), // XBUTTON1
            "Mouse5" => button = Some(2), // XBUTTON2
            _ => return None,
        }
    }

    button.map(|b| (b, ctrl, alt, shift))
}

/// Low-level mouse hook procedure
#[cfg(windows)]
unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_XBUTTONDOWN || msg == WM_XBUTTONUP {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let xbutton = (info.mouseData >> 16) as u16;

            // Read config under lock, clone AppHandle, release lock before calling handler
            let matched = {
                let state = get_state().lock();
                if let Some(ref config) = state.config {
                    if xbutton == config.target_button {
                        let ctrl_ok = !config.require_ctrl
                            || (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0);
                        let alt_ok = !config.require_alt
                            || (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0);
                        let shift_ok = !config.require_shift
                            || (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0);

                        if ctrl_ok && alt_ok && shift_ok {
                            Some((config.app_handle.clone(), msg == WM_XBUTTONDOWN))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((app, is_pressed)) = matched {
                super::handle_mouse_event(&app, is_pressed);
                // Consume the event to prevent browser back/forward
                return LRESULT(1);
            }
        }
    }

    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

/// Install the mouse hook for a mouse button shortcut
#[cfg(windows)]
pub fn install(shortcut: &str, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let (button, ctrl, alt, shift) = parse_mouse_shortcut(shortcut)
        .ok_or_else(|| format!("Invalid mouse shortcut: {}", shortcut))?;

    // Uninstall any existing hook first
    uninstall();

    // Set config before starting the thread
    {
        let mut state = get_state().lock();
        state.config = Some(MouseHookConfig {
            target_button: button,
            require_ctrl: ctrl,
            require_alt: alt,
            require_shift: shift,
            app_handle,
        });
    }

    // Spawn a dedicated thread for the mouse hook message pump
    let thread = std::thread::Builder::new()
        .name("mouse-hook".to_string())
        .spawn(move || {
            unsafe {
                let hook =
                    SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), HINSTANCE::default(), 0);

                match hook {
                    Ok(hook) => {
                        {
                            let mut state = get_state().lock();
                            state.thread_id =
                                Some(windows::Win32::System::Threading::GetCurrentThreadId());
                        }

                        tracing::debug!("Mouse hook thread started, entering message pump");

                        // Message pump - required for low-level hooks
                        let mut msg = MSG::default();
                        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        // Cleanup
                        let _ = UnhookWindowsHookEx(hook);
                        tracing::debug!("Mouse hook thread exiting");
                    }
                    Err(e) => {
                        tracing::error!("Failed to install mouse hook: {}", e);
                    }
                }
            }
        })?;

    // Store the thread handle
    {
        let mut state = get_state().lock();
        state.thread_handle = Some(thread);
    }

    // Give the hook thread time to install
    std::thread::sleep(std::time::Duration::from_millis(50));

    tracing::info!("Mouse hook installed for shortcut: {}", shortcut);
    Ok(())
}

/// Uninstall the mouse hook
#[cfg(windows)]
pub fn uninstall() {
    let (thread_id, thread_handle) = {
        let mut state = get_state().lock();
        state.config = None;
        (state.thread_id.take(), state.thread_handle.take())
    };

    if let Some(tid) = thread_id {
        unsafe {
            // Post WM_QUIT to stop the message pump
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }

        // Wait for the thread to exit
        if let Some(handle) = thread_handle {
            let _ = handle.join();
        }

        tracing::info!("Mouse hook uninstalled");
    }
}

// No-op implementations for non-Windows platforms
#[cfg(not(windows))]
pub fn install(
    _shortcut: &str,
    _app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Mouse button shortcuts are only supported on Windows".into())
}

#[cfg(not(windows))]
pub fn uninstall() {}
