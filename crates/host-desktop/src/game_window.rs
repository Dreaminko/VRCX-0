#[cfg(target_os = "windows")]
use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;

#[cfg(target_os = "windows")]
static FOCUS_REQUEST_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn request_focus_vrchat_window(process_id: Option<u32>) {
    if FOCUS_REQUEST_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return;
    }
    let guard = FocusRequestGuard;
    let spawned = std::thread::Builder::new()
        .name("vrchat-window-focus".into())
        .spawn(move || {
            let _guard = guard;
            focus_vrchat_window(process_id);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "Failed to spawn the VRChat window focus thread");
    }
}

#[cfg(target_os = "windows")]
struct FocusRequestGuard;

#[cfg(target_os = "windows")]
impl Drop for FocusRequestGuard {
    fn drop(&mut self) {
        FOCUS_REQUEST_IN_FLIGHT.store(false, Ordering::Release);
    }
}

#[cfg(target_os = "windows")]
fn focus_vrchat_window(process_id: Option<u32>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindowAsync, SW_RESTORE,
    };

    let process_ids =
        resolve_focus_process_ids(process_id, crate::process_status::vrchat_process_ids());
    if process_ids.is_empty() {
        return;
    }

    let Some(window) = find_vrchat_window(process_ids) else {
        return;
    };

    unsafe {
        if IsIconic(window) != 0 {
            ShowWindowAsync(window, SW_RESTORE);
        }
        SetForegroundWindow(window);
    }
}

#[cfg(target_os = "windows")]
fn resolve_focus_process_ids(process_id: Option<u32>, running: Vec<u32>) -> HashSet<u32> {
    match process_id {
        Some(process_id) if running.contains(&process_id) => HashSet::from([process_id]),
        Some(_) => HashSet::new(),
        None => running.into_iter().collect(),
    }
}

#[cfg(target_os = "windows")]
fn find_vrchat_window(process_ids: HashSet<u32>) -> Option<HWND> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::LPARAM;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
    };

    struct WindowSearch {
        process_ids: HashSet<u32>,
        window: HWND,
    }

    unsafe extern "system" fn find_window(window: HWND, parameter: LPARAM) -> BOOL {
        let search = unsafe { &mut *(parameter as *mut WindowSearch) };
        if unsafe { IsWindowVisible(window) } == 0
            || !unsafe { GetWindow(window, GW_OWNER) }.is_null()
        {
            return 1;
        }

        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(window, &mut process_id) };
        if !search.process_ids.contains(&process_id) {
            return 1;
        }

        search.window = window;
        0
    }

    let mut search = WindowSearch {
        process_ids,
        window: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(find_window),
            &mut search as *mut WindowSearch as LPARAM,
        );
    }
    if search.window.is_null() {
        None
    } else {
        Some(search.window)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn request_focus_vrchat_window(_process_id: Option<u32>) {}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{resolve_focus_process_ids, HashSet};

    #[test]
    fn focuses_only_the_launch_pipe_owner_when_several_vrchat_processes_run() {
        assert_eq!(
            resolve_focus_process_ids(Some(200), vec![100, 200, 300]),
            HashSet::from([200])
        );
    }

    #[test]
    fn skips_focus_when_the_launch_pipe_owner_is_not_a_vrchat_process() {
        assert!(resolve_focus_process_ids(Some(999), vec![100, 200]).is_empty());
    }

    #[test]
    fn falls_back_to_every_vrchat_process_when_the_pipe_owner_is_unknown() {
        assert_eq!(
            resolve_focus_process_ids(None, vec![100, 200]),
            HashSet::from([100, 200])
        );
    }

    #[test]
    fn skips_focus_when_vrchat_is_not_running() {
        assert!(resolve_focus_process_ids(None, Vec::new()).is_empty());
    }
}
