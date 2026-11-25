use parking_lot::Mutex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Exact JSON schema decoded by `apps/server/src/bootstrap.ts` (`DesktopBackendBootstrap`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopBackendBootstrapPayload {
    mode: &'static str,
    no_browser: bool,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    t3_home: Option<String>,
    host: &'static str,
    desktop_bootstrap_token: String,
    tailscale_serve_enabled: bool,
    tailscale_serve_port: u16,
    desktop_app_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBootstrapInfo {
    pub id: String,
    pub label: String,
    pub running_distro: Option<String>,
    pub http_base_url: String,
    pub ws_base_url: String,
    pub bootstrap_token: String,
}

pub struct LocalBackendManager {
    info: LocalBootstrapInfo,
    home_dir: PathBuf,
    child: Mutex<Option<Child>>,
}

impl LocalBackendManager {
    /// Spawns `mycode-server` strictly bound to `127.0.0.1` on an ephemeral loopback port,
    /// writes the NDJSON `DesktopBackendBootstrap` envelope on stdin (`--bootstrap-fd 0`),
    /// and waits for `/.well-known/mycode/environment` to return HTTP 200.
    pub fn start(resource_dir: Option<PathBuf>) -> Result<Self, String> {
        let port = allocate_loopback_port()?;
        let token = generate_bootstrap_token();
        let home_dir = resolve_mycode_home();
        let _ = fs::create_dir_all(&home_dir);

        let server_entry = resolve_server_entry(resource_dir.as_deref())?;
        let node_bin = resolve_node_executable();

        let mut cmd = Command::new(&node_bin);
        cmd.arg(&server_entry)
            .args([
                "--mode",
                "desktop",
                "--no-browser",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--bootstrap-fd",
                "0",
            ])
            .env("MYCODE_HOST", "127.0.0.1")
            .env("MYCODE_NO_BROWSER", "1")
            .env("MYCODE_MODE", "desktop")
            .env("MYCODE_HOME", &home_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(parent) = server_entry.parent().and_then(|p| p.parent()) {
            cmd.current_dir(parent);
        }

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn local MyCode server ({} {}): {e}",
                node_bin.display(),
                server_entry.display()
            )
        })?;

        let payload = DesktopBackendBootstrapPayload {
            mode: "desktop",
            no_browser: true,
            port,
            t3_home: Some(home_dir.to_string_lossy().into_owned()),
            host: "127.0.0.1",
            desktop_bootstrap_token: token.clone(),
            tailscale_serve_enabled: false,
            tailscale_serve_port: 443,
            desktop_app_update: false,
        };

        if let Some(mut stdin) = child.stdin.take() {
            let line = format!(
                "{}\n",
                serde_json::to_string(&payload).map_err(|e| e.to_string())?
            );
            let _ = stdin.write_all(line.as_bytes());
            let _ = stdin.flush();
        }

        wait_for_loopback_readiness(port, &mut child, Duration::from_secs(45))?;

        let info = LocalBootstrapInfo {
            id: "primary".to_string(),
            label: "Local".to_string(),
            running_distro: None,
            http_base_url: format!("http://127.0.0.1:{port}/"),
            ws_base_url: format!("ws://127.0.0.1:{port}/"),
            bootstrap_token: token,
        };

        Ok(Self {
            info,
            home_dir,
            child: Mutex::new(Some(child)),
        })
    }

    pub fn bootstrap_info(&self) -> LocalBootstrapInfo {
        self.info.clone()
    }

    pub fn read_host_environment_id(&self) -> Option<String> {
        let path = self.home_dir.join("state").join("environment-id");
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Exchanges the desktop bootstrap token for a local bearer access token over loopback HTTP.
    pub fn exchange_bearer_token(&self) -> Result<String, String> {
        let port = url::Url::parse(&self.info.http_base_url)
            .ok()
            .and_then(|u| u.port())
            .ok_or_else(|| "Invalid local backend URL".to_string())?;
        let body = serde_json::json!({
            "credential": self.info.bootstrap_token,
        })
        .to_string();

        let (status, resp_body) = loopback_http_request(
            port,
            "POST",
            "/api/auth/bootstrap/bearer",
            Some(&body),
            Some("application/json"),
        )?;

        if status == 200
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&resp_body)
            && let Some(token) = parsed.get("token").and_then(|v| v.as_str())
        {
            return Ok(token.to_string());
        }
        Ok(self.info.bootstrap_token.clone())
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.child.lock().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for LocalBackendManager {
    fn drop(&mut self) {
        self.stop();
    }
}

fn allocate_loopback_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to allocate local loopback port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();
    drop(listener);
    Ok(port)
}

fn generate_bootstrap_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let mut out = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn resolve_mycode_home() -> PathBuf {
    if let Ok(explicit) = std::env::var("MYCODE_HOME")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".mycode")
}

fn resolve_node_executable() -> PathBuf {
    if let Ok(explicit) = std::env::var("MYCODE_NODE_PATH")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit.trim());
    }
    PathBuf::from("node")
}

fn resolve_server_entry(resource_dir: Option<&Path>) -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("MYCODE_SERVER_ENTRY")
        && !explicit.trim().is_empty()
    {
        let p = PathBuf::from(explicit.trim());
        if p.is_file() {
            return Ok(p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = resource_dir {
        candidates.push(res.join("server").join("dist").join("bin.mjs"));
        candidates.push(res.join("bin.mjs"));
    }
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().take(6) {
            candidates.push(ancestor.join("apps").join("server").join("dist").join("bin.mjs"));
            candidates.push(ancestor.join("server").join("dist").join("bin.mjs"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors().take(4) {
            candidates.push(ancestor.join("apps").join("server").join("dist").join("bin.mjs"));
        }
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(repo_root) = manifest_dir.parent().and_then(|p| p.parent()) {
        candidates.push(repo_root.join("apps").join("server").join("dist").join("bin.mjs"));
    }

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }

    Err(format!(
        "Could not locate mycode-server entry (`apps/server/dist/bin.mjs`). Checked: {:?}",
        candidates
    ))
}

fn wait_for_loopback_readiness(
    port: u16,
    child: &mut Child,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!(
                "Local MyCode server exited prematurely with status {status}"
            ));
        }
        if let Ok((200, _)) =
            loopback_http_request(port, "GET", "/.well-known/mycode/environment", None, None)
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(120));
    }
    Err(format!(
        "Timed out waiting for local MyCode server on 127.0.0.1:{port}"
    ))
}

/// Minimal, zero-TLS HTTP/1.1 client restricted exclusively to `127.0.0.1`.
pub fn loopback_http_request(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
    content_type: Option<&str>,
) -> Result<(u16, String), String> {
    let (status, _ct, bytes) =
        loopback_http_request_raw(port, method, path, body.map(str::as_bytes), content_type)?;
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

pub fn loopback_http_request_raw(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    content_type: Option<&str>,
) -> Result<(u16, String, Vec<u8>), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://tauri.localhost\r\nConnection: close\r\n"
    );
    if let Some(ct) = content_type {
        req.push_str(&format!("Content-Type: {ct}\r\n"));
    }
    if let Some(payload) = body {
        req.push_str(&format!("Content-Length: {}\r\n", payload.len()));
    }
    req.push_str("\r\n");

    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    if let Some(payload) = body {
        stream.write_all(payload).map_err(|e| e.to_string())?;
    }
    stream.flush().map_err(|e| e.to_string())?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;

    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "Malformed HTTP response".to_string())?;
    let header_str = String::from_utf8_lossy(&raw[..split]);
    let mut lines = header_str.lines();
    let status_line = lines.next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(500);

    let mut resp_content_type = "application/octet-stream".to_string();
    let mut is_chunked = false;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            if key == "content-type" {
                resp_content_type = val.split(';').next().unwrap_or(val).trim().to_string();
            } else if key == "transfer-encoding" && val.to_ascii_lowercase().contains("chunked") {
                is_chunked = true;
            }
        }
    }

    let body_slice = &raw[split + 4..];
    let body_bytes = if is_chunked {
        decode_chunked(body_slice)
    } else {
        body_slice.to_vec()
    };

    Ok((status, resp_content_type, body_bytes))
}

fn decode_chunked(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let Some(rel_end) = input[pos..].windows(2).position(|w| w == b"\r\n") else {
            break;
        };
        let size_str = String::from_utf8_lossy(&input[pos..pos + rel_end]);
        let size_hex = size_str.split(';').next().unwrap_or("").trim();
        let Ok(chunk_len) = usize::from_str_radix(size_hex, 16) else {
            break;
        };
        pos += rel_end + 2;
        if chunk_len == 0 || pos + chunk_len > input.len() {
            break;
        }
        out.extend_from_slice(&input[pos..pos + chunk_len]);
        pos += chunk_len + 2;
    }
    out
}
