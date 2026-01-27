//! Focus Detection
//!
//! Detect active window and text input fields.

/// Information about the active window
#[derive(Debug, Clone)]
pub struct ActiveWindow {
    /// Window title
    pub title: String,
    /// Process name
    pub process_name: String,
    /// Whether a text input is focused
    pub has_text_input: bool,
}

/// Get information about the currently active window
#[cfg(windows)]
pub fn get_active_window() -> Option<ActiveWindow> {
    use windows::Win32::UI::WindowsAndMessaging::*;

    // SAFETY: These Windows API calls are safe because:
    // - GetForegroundWindow returns a valid HWND or null (we check for null)
    // - GetWindowTextW reads into a stack-allocated buffer of known size
    // - GetWindowThreadProcessId writes to a valid mutable reference
    // - All handles and buffers are properly sized and aligned
    // - No memory is allocated that requires manual deallocation
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return None;
        }

        // Get window title
        let mut title_buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

        // Get process name
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let process_name = get_process_name(pid).unwrap_or_default();

        // Check for text input
        let has_text_input = check_text_input(hwnd);

        Some(ActiveWindow {
            title,
            process_name,
            has_text_input,
        })
    }
}

#[cfg(not(windows))]
pub fn get_active_window() -> Option<ActiveWindow> {
    None
}

/// Get process name from PID
#[cfg(windows)]
fn get_process_name(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    // SAFETY: These Windows API calls are safe because:
    // - OpenProcess returns a valid handle or fails (we use .ok()? to handle failure)
    // - GetModuleBaseNameW reads into a stack-allocated buffer of known size
    // - CloseHandle properly releases the process handle we obtained
    // - The handle is closed in all code paths (success or early return)
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut name_buf = [0u16; 256];
        let len = GetModuleBaseNameW(handle, None, &mut name_buf);

        let _ = CloseHandle(handle);

        if len > 0 {
            Some(String::from_utf16_lossy(&name_buf[..len as usize]))
        } else {
            None
        }
    }
}

/// Check if current focus is a text input
#[cfg(windows)]
fn check_text_input(_hwnd: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    use windows::Win32::UI::WindowsAndMessaging::*;

    // SAFETY: These Windows API calls are safe because:
    // - GetFocus returns a valid HWND or null (we check for null)
    // - GetClassNameW reads into a stack-allocated buffer of known size
    // - No memory allocation or resource handles require cleanup
    // - Buffer size is sufficient for any Windows class name
    unsafe {
        let focus = GetFocus();
        if focus.0 == std::ptr::null_mut() {
            return false;
        }

        // Get class name of focused control
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(focus, &mut class_buf);
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]).to_lowercase();

        // Common text input class names
        let text_classes = [
            "edit",
            "richedit",
            "richedit20w",
            "richedit50w",
            "textarea",
            "combobox",
            "chrome_widget",
            "mozillawindowclass",
        ];

        text_classes.iter().any(|c| class_name.contains(c))
    }
}

/// Check if the active window likely accepts text input
pub fn has_text_input_focus() -> bool {
    get_active_window()
        .map(|w| w.has_text_input)
        .unwrap_or(false)
}

/// Check if we should auto-paste (any window except GigaWhisper is active)
pub fn should_auto_paste() -> bool {
    match get_active_window() {
        Some(window) => {
            // Don't paste into our own app
            let is_our_app = window.process_name.to_lowercase().contains("gigawhisper")
                || window.title.to_lowercase().contains("gigawhisper");
            !is_our_app
        }
        None => false,
    }
}

/// Alias for should_auto_paste (more permissive than has_text_input_focus)
pub fn has_text_focus() -> bool {
    should_auto_paste()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_window() {
        // This test may return None on CI without display
        let window = get_active_window();
        // Just ensure it doesn't panic
        let _ = window;
    }
}
