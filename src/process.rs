use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

/// Windows sharing mode is an OS lease: released even after a crashed process.
pub fn gpu_lease(gpu: u32) -> Result<File> {
    let directory = std::env::current_dir()?.join(".tools/gpu-locks");
    std::fs::create_dir_all(&directory)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options
        .open(directory.join(format!("gpu-{gpu}.lock")))
        .context("GPU başka bir Popy işi tarafından kullanılıyor veya kilit klasörüne erişilemiyor")
}

pub fn install_cancel_handler() {
    #[cfg(windows)]
    unsafe {
        unsafe extern "system" fn handler(event: u32) -> i32 {
            if event <= 1 {
                CANCELLED.store(true, Ordering::SeqCst);
                1
            } else {
                0
            }
        }
        unsafe extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }
        SetConsoleCtrlHandler(Some(handler), 1);
    }
}

#[derive(Debug, Serialize)]
pub struct Stage {
    pub name: String,
    pub seconds: f64,
    pub peak_process_ram_bytes: Option<usize>,
}

/// Logs go straight to files: child stdout/stderr cannot fill a pipe and deadlock.
pub fn run(command: &mut Command, label: &str, log_dir: &Path, timeout: Duration) -> Result<Stage> {
    if CANCELLED.load(Ordering::SeqCst) {
        bail!("İşlem iptal edildi.");
    }
    let stdout_path = log_dir.join(format!("{label}.stdout.log"));
    let stderr_path = log_dir.join(format!("{label}.stderr.log"));
    command
        .stdin(Stdio::null())
        .stdout(File::create(&stdout_path)?)
        .stderr(File::create(&stderr_path)?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Separate child group: parent handles Ctrl+C and terminates the child itself.
        command.creation_flags(0x0000_0200 | 0x0800_0000);
    }
    eprintln!("[{label}] başladı");
    let started = Instant::now();
    let mut child = command
        .spawn()
        .with_context(|| format!("{label} başlatılamadı"))?;
    let mut peak = 0;
    loop {
        if let Some(bytes) = peak_ram(&child) {
            peak = peak.max(bytes);
        }
        if CANCELLED.load(Ordering::SeqCst) || started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "{label}: iptal veya zaman sınırı. Günlükler: {}",
                log_dir.display()
            );
        }
        if let Some(status) = child.try_wait()? {
            let seconds = started.elapsed().as_secs_f64();
            if !status.success() {
                let error = std::fs::read_to_string(&stderr_path).unwrap_or_default();
                let tail: String = error
                    .chars()
                    .rev()
                    .take(4000)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                bail!("{label} başarısız ({status}): {tail}");
            }
            eprintln!("[{label}] {seconds:.2} sn");
            return Ok(Stage {
                name: label.into(),
                seconds,
                peak_process_ram_bytes: (peak > 0).then_some(peak),
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(windows)]
fn peak_ram(child: &std::process::Child) -> Option<usize> {
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    struct Counters {
        cb: u32,
        page_faults: u32,
        values: [usize; 8],
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        page_faults: 0,
        values: [0; 8],
    };
    let result = unsafe { GetProcessMemoryInfo(child.as_raw_handle(), &mut counters, counters.cb) };
    (result != 0).then_some(counters.values[0])
}

#[cfg(not(windows))]
fn peak_ram(_: &std::process::Child) -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn only_one_job_can_hold_a_gpu_lease() {
        let lease = gpu_lease(15).unwrap();
        assert!(gpu_lease(15).is_err());
        drop(lease);
        assert!(gpu_lease(15).is_ok());
    }

    #[test]
    #[ignore = "subprocess target, launched by timeout test"]
    fn slow_child() {
        std::thread::sleep(Duration::from_secs(20));
    }

    #[test]
    fn timed_out_child_is_terminated_and_reaped() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::current_dir()
            .unwrap()
            .join("target/test-logs")
            .join(format!("timeout-{stamp}"));
        std::fs::create_dir_all(&directory).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "process::tests::slow_child", "--ignored"]);
        let start = Instant::now();
        assert!(
            run(
                &mut command,
                "timeout-test",
                &directory,
                Duration::from_millis(200)
            )
            .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(directory.join("timeout-test.stderr.log").exists());
    }
}
