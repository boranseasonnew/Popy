use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

static CANCELLED: AtomicBool = AtomicBool::new(false);
static CANCEL_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_cancel_file(path: Option<PathBuf>) {
    *CANCEL_FILE.lock().expect("cancel lock") = path;
}

pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
        || CANCEL_FILE
            .lock()
            .expect("cancel lock")
            .as_ref()
            .is_some_and(|p| p.is_file())
}

pub fn check_cancelled() -> Result<()> {
    if is_cancelled() {
        bail!("İşlem iptal edildi.");
    }
    Ok(())
}

/// Every child is reaped on error. On Windows a kill-on-close Job Object also
/// terminates descendants if the GUI/worker crashes or is forcibly closed.
pub struct ManagedChild {
    pub child: Child,
    #[cfg(windows)]
    _job: Job,
}

impl ManagedChild {
    pub fn spawn(command: &mut Command) -> Result<Self> {
        check_cancelled()?;
        command.stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0000_0200 | 0x0800_0000);
        }
        let mut child = command.spawn().context("Alt süreç başlatılamadı")?;
        #[cfg(windows)]
        let job = match Job::attach(&child) {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        Ok(Self {
            child,
            #[cfg(windows)]
            _job: job,
        })
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Job {
    fn attach(child: &Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let job = Self(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                return Err(std::io::Error::last_os_error())
                    .context("Alt süreç yaşam döngüsü denetlenemedi");
            }
            Ok(job)
        }
    }
}

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Stream ffprobe records through a bounded channel: long CFR scans do not keep
/// every timestamp in memory. The consumer checks cancellation and a deadline.
pub fn capture_lines(
    command: &mut Command,
    timeout: Duration,
    mut consume: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut managed = ManagedChild::spawn(command)?;
    let stdout = managed.child.stdout.take().context("stdout yok")?;
    let stderr = managed.child.stderr.take().context("stderr yok")?;
    let errors = std::thread::spawn(move || {
        let mut tail = Vec::new();
        let _ = stderr.take(65536).read_to_end(&mut tail);
        String::from_utf8_lossy(&tail).into_owned()
    });
    let (send, receive) = mpsc::sync_channel(64);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut bytes = Vec::new();
            let result = reader
                .by_ref()
                .take(1_048_577)
                .read_until(b'\n', &mut bytes);
            match result {
                Ok(0) => break,
                Ok(_) if bytes.len() <= 1_048_576 => {
                    if send.send(Ok(bytes)).is_err() {
                        break;
                    }
                }
                _ => {
                    let _ = send.send(Err("Araç satırı okunamadı veya 1 MiB sınırını aştı"));
                    break;
                }
            }
        }
    });
    let started = Instant::now();
    loop {
        check_cancelled()?;
        if started.elapsed() > timeout {
            bail!("Araç sorgusu zaman sınırını aştı");
        }
        match receive.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(bytes)) => consume(std::str::from_utf8(&bytes)?.trim_end())?,
            Ok(Err(error)) => bail!("{error}"),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    loop {
        check_cancelled()?;
        if started.elapsed() > timeout {
            bail!("Araç sorgusu zaman sınırını aştı");
        }
        if let Some(status) = managed.child.try_wait()? {
            let errors = errors.join().unwrap_or_default();
            if !status.success() {
                bail!("Araç sorgusu başarısız: {errors}");
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn capture(command: &mut Command) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    capture_lines(command, Duration::from_secs(120), |line| {
        anyhow::ensure!(
            bytes.len() + line.len() < 16_000_000,
            "Araç yanıtı 16 MB sınırını aştı"
        );
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        Ok(())
    })?;
    Ok(bytes)
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
    check_cancelled()?;
    let stdout_path = log_dir.join(format!("{label}.stdout.log"));
    let stderr_path = log_dir.join(format!("{label}.stderr.log"));
    command
        .stdin(Stdio::null())
        .stdout(File::create(&stdout_path)?)
        .stderr(File::create(&stderr_path)?);
    eprintln!("[{label}] başladı");
    let started = Instant::now();
    let mut managed =
        ManagedChild::spawn(command).with_context(|| format!("{label} başlatılamadı"))?;
    let child = &mut managed.child;
    let mut peak = 0;
    loop {
        if let Some(bytes) = peak_ram(child) {
            peak = peak.max(bytes);
        }
        if is_cancelled() || started.elapsed() > timeout {
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

    #[test]
    fn timestamp_capture_has_a_deadline_too() {
        let start = Instant::now();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "process::tests::slow_child", "--ignored"]);
        assert!(capture_lines(&mut command, Duration::from_millis(200), |_| Ok(())).is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "subprocess target, launched by descendant supervision test"]
    fn descendant_parent() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process::tests::slow_child", "--ignored"])
            .spawn()
            .unwrap();
        std::fs::write(
            std::env::var_os("POPY_TEST_PID").unwrap(),
            child.id().to_string(),
        )
        .unwrap();
        let _ = child.wait();
    }

    #[cfg(windows)]
    #[test]
    fn dropping_worker_job_terminates_its_descendant() {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let directory = std::env::current_dir().unwrap().join("target/test-logs");
        std::fs::create_dir_all(&directory).unwrap();
        let pid_file = directory.join(format!(
            "descendant-{}.pid",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "process::tests::descendant_parent", "--ignored"])
            .env("POPY_TEST_PID", &pid_file)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let managed = ManagedChild::spawn(&mut command).unwrap();
        let start = Instant::now();
        while !pid_file.is_file() && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        let pid: u32 = std::fs::read_to_string(&pid_file).unwrap().parse().unwrap();
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        assert!(!handle.is_null());
        drop(managed);
        let result = unsafe { WaitForSingleObject(handle, 5000) };
        unsafe {
            CloseHandle(handle);
        }
        assert_eq!(
            result, WAIT_OBJECT_0,
            "Descendant survived worker job closure"
        );
    }
}
