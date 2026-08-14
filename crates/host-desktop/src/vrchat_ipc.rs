#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VrcIpcSendResult {
    pub accepted: bool,
    pub server_process_id: Option<u32>,
}

#[cfg(any(target_os = "linux", test))]
const LINUX_LAUNCH_FORWARD_SCRIPT: &str =
    "while IFS= read -r -d '' kv; do export \"$kv\"; done < \"/proc/$1/environ\"; exec wine \"$2\" \"$3\"";

pub fn vrcipc_send(message: &str) -> bool {
    vrcipc_send_with_result(message).accepted
}

#[cfg(target_os = "windows")]
pub fn vrcipc_send_with_result(message: &str) -> VrcIpcSendResult {
    use std::io::{Read, Write};
    use std::os::windows::io::AsRawHandle;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;

    let pipe_path = r"\\.\pipe\VRChatURLLaunchPipe";

    let mut pipe = match open_pipe_client(pipe_path, Duration::from_secs(1)) {
        Some(p) => p,
        None => return VrcIpcSendResult::default(),
    };
    let mut server_process_id = 0;
    let server_process_id = if unsafe {
        GetNamedPipeServerProcessId(pipe.as_raw_handle() as HANDLE, &mut server_process_id)
    } != 0
    {
        Some(server_process_id)
    } else {
        None
    };

    let bytes = message.as_bytes();
    if pipe.write_all(bytes).is_err() {
        return VrcIpcSendResult {
            accepted: false,
            server_process_id,
        };
    }

    let mut result = [0u8; 1];
    if pipe.read_exact(&mut result).is_err() {
        return VrcIpcSendResult {
            accepted: false,
            server_process_id,
        };
    }

    VrcIpcSendResult {
        accepted: result[0] == 1,
        server_process_id,
    }
}

#[cfg(target_os = "windows")]
fn open_pipe_client(pipe_path: &str, timeout: std::time::Duration) -> Option<std::fs::File> {
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Storage::FileSystem::*;
    use windows_sys::Win32::System::Pipes::*;

    let wide: Vec<u16> = pipe_path.encode_utf16().chain(std::iter::once(0)).collect();
    let deadline = std::time::Instant::now() + timeout;

    loop {
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut() as HANDLE,
            )
        };

        if handle != INVALID_HANDLE_VALUE {
            use std::os::windows::io::FromRawHandle;
            return Some(unsafe { std::fs::File::from_raw_handle(handle) });
        }

        if std::time::Instant::now() >= deadline {
            return None;
        }

        let ok = unsafe { WaitNamedPipeW(wide.as_ptr(), 1000) };
        if ok == 0 && std::time::Instant::now() >= deadline {
            return None;
        }
    }
}

#[cfg(target_os = "linux")]
pub fn vrcipc_send_with_result(message: &str) -> VrcIpcSendResult {
    let accepted = match linux_vrcipc_send(message) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(%error, "Linux VRChat launch pipe bridge failed");
            false
        }
    };
    VrcIpcSendResult {
        accepted,
        server_process_id: None,
    }
}

#[cfg(target_os = "linux")]
fn linux_vrcipc_send(message: &str) -> Result<bool, String> {
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc;

    let process_id = crate::process_status::linux_vrchat_process_id()
        .ok_or_else(|| "VRChat process not found".to_string())?;
    let paths = crate::vrchat_paths::discover_linux_vrchat_paths()?;
    let launch_exe = paths.install_path.join("launch.exe");
    if !launch_exe.is_file() {
        return Err(format!(
            "VRChat launch.exe not found at {}",
            launch_exe.display()
        ));
    }

    let (child_sender, child_receiver) = mpsc::sync_channel::<Child>(1);
    std::thread::Builder::new()
        .name("vrchat-launch-forward".into())
        .spawn(move || {
            let Ok(mut child) = child_receiver.recv() else {
                return;
            };
            match child.wait() {
                Ok(status) if !status.success() => {
                    tracing::warn!(%status, "Linux VRChat launch forwarder exited unsuccessfully");
                }
                Err(error) => {
                    tracing::warn!(%error, "Failed to wait for Linux VRChat launch forwarder");
                }
                _ => {}
            }
        })
        .map_err(|error| format!("start VRChat launch forwarder reaper: {error}"))?;

    let mut command = Command::new("nsenter");
    command
        .args(linux_launch_forward_arguments(
            process_id,
            &launch_exe,
            message,
        ))
        .stdin(Stdio::null());
    let child = command
        .spawn()
        .map_err(|error| format!("start VRChat launch forwarder: {error}"))?;
    child_sender
        .send(child)
        .map_err(|_| "VRChat launch forwarder reaper stopped unexpectedly".to_string())?;
    Ok(true)
}

#[cfg(any(target_os = "linux", test))]
fn linux_launch_forward_arguments(
    process_id: u32,
    launch_exe: &std::path::Path,
    launch_url: &str,
) -> Vec<std::ffi::OsString> {
    let process_id = process_id.to_string();
    let launch_url = if launch_url.contains("attach=1") {
        launch_url.to_string()
    } else {
        format!("{launch_url}&attach=1")
    };
    [
        std::ffi::OsString::from("-t"),
        std::ffi::OsString::from(&process_id),
        std::ffi::OsString::from("-U"),
        std::ffi::OsString::from("-m"),
        std::ffi::OsString::from("--preserve-credentials"),
        std::ffi::OsString::from("--"),
        std::ffi::OsString::from("/bin/bash"),
        std::ffi::OsString::from("-c"),
        std::ffi::OsString::from(LINUX_LAUNCH_FORWARD_SCRIPT),
        std::ffi::OsString::from("_"),
        std::ffi::OsString::from(process_id),
        launch_exe.as_os_str().to_owned(),
        std::ffi::OsString::from(launch_url),
    ]
    .into()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn vrcipc_send_with_result(_message: &str) -> VrcIpcSendResult {
    VrcIpcSendResult::default()
}

#[cfg(test)]
mod linux_tests {
    use std::ffi::OsString;
    use std::path::Path;

    use super::{linux_launch_forward_arguments, LINUX_LAUNCH_FORWARD_SCRIPT};

    #[test]
    fn builds_namespace_launch_arguments_with_attach_mode() {
        assert_eq!(
            linux_launch_forward_arguments(
                42,
                Path::new("/games/Steam Library/VRChat/launch.exe"),
                "vrchat://launch?id=wrld_1:2"
            ),
            vec![
                OsString::from("-t"),
                OsString::from("42"),
                OsString::from("-U"),
                OsString::from("-m"),
                OsString::from("--preserve-credentials"),
                OsString::from("--"),
                OsString::from("/bin/bash"),
                OsString::from("-c"),
                OsString::from(LINUX_LAUNCH_FORWARD_SCRIPT),
                OsString::from("_"),
                OsString::from("42"),
                OsString::from("/games/Steam Library/VRChat/launch.exe"),
                OsString::from("vrchat://launch?id=wrld_1:2&attach=1"),
            ]
        );
    }

    #[test]
    fn preserves_existing_attach_mode() {
        let arguments = linux_launch_forward_arguments(
            42,
            Path::new("/games/VRChat/launch.exe"),
            "vrchat://launch?id=wrld_1:2&attach=1",
        );

        assert_eq!(
            arguments.last(),
            Some(&OsString::from("vrchat://launch?id=wrld_1:2&attach=1"))
        );
    }
}
