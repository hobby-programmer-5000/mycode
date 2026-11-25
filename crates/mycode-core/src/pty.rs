use parking_lot::Mutex;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("PTY operation failed: {0}")]
    Runtime(String),
    #[error("PTY session not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnRequest {
    pub shell: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnInfo {
    pub session_id: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyPollResult {
    pub data: Vec<u8>,
    pub exited: bool,
    pub exit_code: Option<u32>,
}

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    buffer: Arc<Mutex<Vec<u8>>>,
    eof: Arc<AtomicBool>,
    exit_code: Arc<Mutex<Option<u32>>>,
}

/// Thread-safe Rust ConPTY / Unix PTY session manager built on `portable-pty`.
#[derive(Default)]
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtySession>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&self, req: PtySpawnRequest) -> Result<PtySpawnInfo, PtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: req.rows.max(1),
                cols: req.cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Runtime(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&req.shell);
        for arg in &req.args {
            cmd.arg(arg);
        }
        if !req.cwd.is_empty() {
            cmd.cwd(&req.cwd);
        }
        for (k, v) in &req.env {
            cmd.env(k, v);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Runtime(e.to_string()))?;
        let pid = child.process_id();
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Runtime(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Runtime(e.to_string()))?;

        let buffer = Arc::new(Mutex::new(Vec::<u8>::with_capacity(8192)));
        let eof = Arc::new(AtomicBool::new(false));
        let exit_code = Arc::new(Mutex::new(None));

        let buf_clone = Arc::clone(&buffer);
        let eof_clone = Arc::clone(&eof);

        thread::Builder::new()
            .name("mycode-pty-reader".to_string())
            .spawn(move || {
                let mut chunk = [0u8; 8192];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => {
                            eof_clone.store(true, Ordering::Release);
                            break;
                        }
                        Ok(n) => {
                            let mut guard = buf_clone.lock();
                            // Cap unread buffer at 4 MiB to prevent runaway memory growth.
                            if guard.len() + n <= 4 * 1024 * 1024 {
                                guard.extend_from_slice(&chunk[..n]);
                            }
                        }
                    }
                }
            })
            .map_err(|e| PtyError::Runtime(e.to_string()))?;

        let session_id = Uuid::new_v4().to_string();
        self.sessions.lock().insert(
            session_id.clone(),
            PtySession {
                master: pair.master,
                writer,
                child,
                buffer,
                eof,
                exit_code,
            },
        );

        Ok(PtySpawnInfo { session_id, pid })
    }

    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), PtyError> {
        let mut sessions = self.sessions.lock();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))?;
        session
            .writer
            .write_all(data)
            .and_then(|()| session.writer.flush())
            .map_err(|e| PtyError::Runtime(e.to_string()))
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), PtyError> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))?;
        session
            .master
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Runtime(e.to_string()))
    }

    pub fn poll(&self, session_id: &str) -> Result<PtyPollResult, PtyError> {
        let mut sessions = self.sessions.lock();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))?;

        let data = {
            let mut guard = session.buffer.lock();
            std::mem::take(&mut *guard)
        };

        let mut exited = session.eof.load(Ordering::Acquire);
        let mut code = *session.exit_code.lock();
        if code.is_none()
            && let Ok(Some(status)) = session.child.try_wait()
        {
            let ec = status.exit_code();
            *session.exit_code.lock() = Some(ec);
            code = Some(ec);
            exited = true;
        }

        Ok(PtyPollResult {
            data,
            exited,
            exit_code: code,
        })
    }

    pub fn kill(&self, session_id: &str) -> Result<(), PtyError> {
        let session = self.sessions.lock().remove(session_id);
        if let Some(mut session) = session {
            // Detach Windows ConPTY handle teardown onto a background reaper thread so
            // ClosePseudoConsole never blocks the caller even if conhost lags during exit.
            let _ = thread::Builder::new()
                .name("mycode-pty-reaper".to_string())
                .spawn(move || {
                    drop(session.writer);
                    let _ = session.child.kill();
                    drop(session.master);
                    let _ = session.child.try_wait();
                });
        }
        Ok(())
    }

    pub fn kill_all(&self) {
        let drained: Vec<PtySession> = self.sessions.lock().drain().map(|(_, s)| s).collect();
        if !drained.is_empty() {
            let _ = thread::Builder::new()
                .name("mycode-pty-reaper".to_string())
                .spawn(move || {
                    for mut session in drained {
                        drop(session.writer);
                        let _ = session.child.kill();
                        drop(session.master);
                        let _ = session.child.try_wait();
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn spawns_and_reads_from_native_pty() {
        let mgr = PtyManager::new();
        #[cfg(windows)]
        let req = PtySpawnRequest {
            shell: "cmd.exe".to_string(),
            args: vec!["/C".to_string(), "echo MYCODE_RUST_PTY_OK".to_string()],
            cwd: std::env::temp_dir().to_string_lossy().into_owned(),
            cols: 80,
            rows: 24,
            env: HashMap::new(),
        };
        #[cfg(not(windows))]
        let req = PtySpawnRequest {
            shell: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "echo MYCODE_RUST_PTY_OK".to_string()],
            cwd: "/tmp".to_string(),
            cols: 80,
            rows: 24,
            env: HashMap::new(),
        };

        let spawned = mgr.spawn(req).expect("spawn native PTY");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut collected = String::new();
        let mut answered_dsr = false;

        while Instant::now() < deadline {
            let polled = mgr.poll(&spawned.session_id).expect("poll PTY");
            if !polled.data.is_empty() {
                collected.push_str(&String::from_utf8_lossy(&polled.data));
            }
            // Windows ConPTY emits a DSR cursor-position query (ESC[6n) before flushing output.
            if !answered_dsr && collected.contains("\x1b[6n") {
                let _ = mgr.write(&spawned.session_id, b"\x1b[1;1R");
                answered_dsr = true;
            }
            if collected.contains("MYCODE_RUST_PTY_OK") {
                break;
            }
            thread::sleep(Duration::from_millis(30));
        }

        let _ = mgr.kill(&spawned.session_id);
        assert!(
            collected.contains("MYCODE_RUST_PTY_OK"),
            "expected PTY output marker, got: {collected:?}"
        );
    }
}
