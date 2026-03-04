//! Mouse Hook for Mouse Button Shortcuts
//!
//! Uses Windows low-level mouse hook (WH_MOUSE_LL) to capture
//! extra mouse buttons (Mouse4/Mouse5) as global shortcuts.
//! This is needed because tauri-plugin-global-shortcut (muda) only
//! supports keyboard shortcuts.
//!
//! IMPORTANT: The hook callback (mouse_hook_proc) runs in the Windows input
//! processing pipeline. If it blocks, ALL mouse input on the entire desktop
//! freezes. Therefore the callback MUST be completely lock-free and return
//! as fast as possible. All heavy work is dispatched asynchronously.

#[cfg(windows)]
use parking_lot::Mutex;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
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

// --- Lock-free hook configuration (read by hook callback without any lock) ---
#[cfg(windows)]
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static HOOK_BUTTON: AtomicU16 = AtomicU16::new(0);
#[cfg(windows)]
static HOOK_REQUIRE_CTRL: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static HOOK_REQUIRE_ALT: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static HOOK_REQUIRE_SHIFT: AtomicBool = AtomicBool::new(false);

// --- AppHandle storage (written during install/uninstall, read via try_lock from hook) ---
#[cfg(windows)]
static APP_HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

#[cfg(windows)]
fn get_app_handle_store() -> &'static Mutex<Option<AppHandle>> {
    APP_HANDLE.get_or_init(|| Mutex::new(None))
}

// --- Thread management (never accessed from hook callback) ---
#[cfg(windows)]
struct ThreadState {
    thread_id: Option<u32>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(windows)]
unsafe impl Send for ThreadState {}

#[cfg(windows)]
static THREAD_STATE: OnceLock<Mutex<ThreadState>> = OnceLock::new();

#[cfg(windows)]
fn get_thread_state() -> &'static Mutex<ThreadState> {
    THREAD_STATE.get_or_init(|| {
        Mutex::new(ThreadState {
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

/// Low-level mouse hook procedure.
///
/// CRITICAL: This function MUST return as fast as possible and NEVER block.
/// Windows calls this in the input processing pipeline - if it blocks,
/// ALL mouse input on the entire desktop freezes until it returns.
#[cfg(windows)]
unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && HOOK_ACTIVE.load(Ordering::Acquire) {
        let msg = wparam.0 as u32;
        if msg == WM_XBUTTONDOWN || msg == WM_XBUTTONUP {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let xbutton = (info.mouseData >> 16) as u16;

            if xbutton == HOOK_BUTTON.load(Ordering::Relaxed) {
                // Check modifier keys (lock-free, just reading key state)
                let ctrl_ok = !HOOK_REQUIRE_CTRL.load(Ordering::Relaxed)
                    || (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0);
                let alt_ok = !HOOK_REQUIRE_ALT.load(Ordering::Relaxed)
                    || (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0);
                let shift_ok = !HOOK_REQUIRE_SHIFT.load(Ordering::Relaxed)
                    || (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0);

                if ctrl_ok && alt_ok && shift_ok {
                    // Get app handle via try_lock - NEVER block here.
                    // If the lock is contended (install/uninstall in progress), skip this event.
                    if let Some(guard) = get_app_handle_store().try_lock() {
                        if let Some(ref app) = *guard {
                            let app_clone = app.clone();
                            let is_pressed = msg == WM_XBUTTONDOWN;
                            // Drop the guard BEFORE spawning to minimize lock hold time
                            drop(guard);
                            // Dispatch asynchronously - NEVER call handle_mouse_event
                            // on the hook thread, as it reads RwLocks that could block.
                            tauri::async_runtime::spawn(async move {
                                super::handle_mouse_event(&app_clone, is_pressed);
                            });
                        }
                    }
                    // Consume the event to prevent browser back/forward
                    return LRESULT(1);
                }
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

    // Set lock-free config (atomics) - readable by hook without any lock
    HOOK_BUTTON.store(button, Ordering::Relaxed);
    HOOK_REQUIRE_CTRL.store(ctrl, Ordering::Relaxed);
    HOOK_REQUIRE_ALT.store(alt, Ordering::Relaxed);
    HOOK_REQUIRE_SHIFT.store(shift, Ordering::Relaxed);

    // Store the app handle
    {
        let mut handle = get_app_handle_store().lock();
        *handle = Some(app_handle);
    }

    // Activate the hook config (release ordering ensures all stores above are visible)
    HOOK_ACTIVE.store(true, Ordering::Release);

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
                            let mut state = get_thread_state().lock();
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
        let mut state = get_thread_state().lock();
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
    // Deactivate the hook first (lock-free, immediate effect)
    HOOK_ACTIVE.store(false, Ordering::Release);

    // Clear the app handle
    {
        let mut handle = get_app_handle_store().lock();
        *handle = None;
    }

    // Stop the hook thread
    let (thread_id, thread_handle) = {
        let mut state = get_thread_state().lock();
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
