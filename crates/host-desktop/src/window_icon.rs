#[cfg(target_os = "windows")]
const APPLICATION_ICON_RESOURCE_ID: u16 = 32512;

#[cfg(target_os = "windows")]
pub fn apply_window_icon(window_handle: isize) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows::Win32::UI::WindowsAndMessaging::{
        LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_SHARED, SM_CXICON,
        SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
    };

    if window_handle == 0 {
        return;
    }

    let window = HWND(window_handle as *mut core::ffi::c_void);
    let dpi = match unsafe { GetDpiForWindow(window) } {
        0 => 96,
        dpi => dpi,
    };
    let module: HINSTANCE = match unsafe { GetModuleHandleW(PCWSTR::null()) } {
        Ok(module) => module.into(),
        Err(error) => {
            tracing::warn!(%error, "Failed to locate the application module for its window icon");
            return;
        }
    };
    let resource = PCWSTR(APPLICATION_ICON_RESOURCE_ID as usize as *const u16);

    for (icon_type, width_metric, height_metric) in [
        (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
        (ICON_BIG, SM_CXICON, SM_CYICON),
    ] {
        let width = unsafe { GetSystemMetricsForDpi(width_metric, dpi) };
        let height = unsafe { GetSystemMetricsForDpi(height_metric, dpi) };
        match unsafe { LoadImageW(Some(module), resource, IMAGE_ICON, width, height, LR_SHARED) } {
            Ok(icon) => unsafe {
                SendMessageW(
                    window,
                    WM_SETICON,
                    Some(WPARAM(icon_type as usize)),
                    Some(LPARAM(icon.0 as isize)),
                );
            },
            Err(error) => {
                tracing::warn!(%error, icon_type, "Failed to load the embedded window icon");
            }
        }
    }
}
