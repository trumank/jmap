use anyhow::Result;

#[cfg(target_os = "windows")]
pub fn get_process_name(pid: i32) -> Result<String> {
    use anyhow::Context;
    use std::mem;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    struct HandleGuard(HANDLE);

    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .context("Failed to get process list snapshot")?;
        let _guard = HandleGuard(snapshot);

        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid as u32 {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());

                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    return Ok(exe_name);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
    }
    anyhow::bail!("Process {pid} not found")
}

#[cfg(target_os = "linux")]
pub fn get_process_name(pid: i32) -> Result<String> {
    use anyhow::Context;

    let maps = proc_maps::get_process_maps(pid)
        .with_context(|| format!("Failed to read proc maps for {pid}"))?;

    for map in maps {
        if let Some(name) = map.filename() {
            let name = name.to_string_lossy();
            if name.ends_with(".exe") {
                return Ok(name
                    .rsplit_once(['\\', '/'])
                    .map(|s| s.1)
                    .unwrap_or(&name)
                    .to_string());
            }
        }
    }
    anyhow::bail!("Failed to find proc exe name for {pid}")
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn get_process_name(_pid: i32) -> Result<String> {
    anyhow::bail!("Unimplemented for target: {}", std::env::consts::OS)
}
