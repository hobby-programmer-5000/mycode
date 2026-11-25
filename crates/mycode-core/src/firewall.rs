use std::net::IpAddr;
use url::Url;

/// Chromium / Edge WebView2 command-line flags enforcing air-gapped loopback-only networking.
///
/// Maps every external DNS lookup to `~NOTFOUND` inside Chromium's network service while allowing
/// local loopback and Tauri's bundled app origin (`tauri.localhost`). OS-spawned CLI providers
/// (Claude Code, Codex, Antigravity) run outside WebView2's network stack and remain unaffected.
pub const WEBVIEW2_OFFLINE_BROWSER_ARGS: &str = concat!(
    "--host-resolver-rules=\"MAP * ~NOTFOUND , ",
    "EXCLUDE localhost , ",
    "EXCLUDE *.localhost , ",
    "EXCLUDE 127.0.0.1 , ",
    "EXCLUDE ::1\" ",
    "--disable-background-networking ",
    "--disable-component-update ",
    "--disable-domain-reliability ",
    "--disable-sync ",
    "--metrics-recording-only"
);

/// Returns true if `hostname` resolves strictly to a local loopback interface or Tauri's virtual
/// localhost origin.
pub fn is_loopback_hostname(hostname: &str) -> bool {
    let normalized = hostname
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();

    if normalized == "localhost"
        || normalized == "tauri.localhost"
        || normalized.ends_with(".localhost")
    {
        return true;
    }

    normalized
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// Returns true if the URL is permitted inside the air-gapped MyCode desktop WebView.
///
/// Allows:
/// - `tauri://localhost/...` and `asset://localhost/...`
/// - `about:blank`, `data:`, `blob:`
/// - `http(s)://` and `ws(s)://` strictly targeting loopback (`127.0.0.1`, `[::1]`, `localhost`, `*.localhost`)
///
/// Blocks every external domain, public IP, LAN IP, `file://`, etc.
pub fn is_allowed_offline_url(raw_url: &str) -> bool {
    let Ok(parsed) = Url::parse(raw_url) else {
        return false;
    };

    match parsed.scheme() {
        "about" => parsed.path() == "blank",
        "data" | "blob" => true,
        "tauri" | "asset" | "mycode" | "mycode-dev" => {
            parsed.host_str().map(is_loopback_hostname).unwrap_or(true)
        }
        "http" | "https" | "ws" | "wss" => parsed
            .host_str()
            .map(is_loopback_hostname)
            .unwrap_or(false),
        _ => false,
    }
}

/// Returns true if `raw_url` is a loopback HTTP(S) URL suitable for local media reads or backend
/// communication.
pub fn is_loopback_http_url(raw_url: &str) -> bool {
    let Ok(parsed) = Url::parse(raw_url) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https")
        && parsed.host_str().map(is_loopback_hostname).unwrap_or(false)
}

/// Validates a user-clicked external URL before opening it in the OS browser/shell.
/// Only allows `http://`, `https://`, or `mailto:` schemes.
pub fn is_safe_external_open_url(raw_url: &str) -> bool {
    let Ok(parsed) = Url::parse(raw_url) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https" | "mailto")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_loopback_and_tauri_origins() {
        for url in [
            "http://127.0.0.1:3773/.well-known/mycode/environment",
            "http://localhost:5733/",
            "http://[::1]:18773/api/health",
            "ws://127.0.0.1:3773/ws?ticket=abc",
            "http://tauri.localhost/index.html",
            "https://tauri.localhost/",
            "tauri://localhost/index.html",
            "about:blank",
            "data:image/png;base64,AAAA",
            "blob:http://tauri.localhost/123",
        ] {
            assert!(is_allowed_offline_url(url), "expected allowed: {url}");
        }
    }

    #[test]
    fn blocks_all_external_and_lan_hosts() {
        for url in [
            "https://github.com/hobby-programmer-5000/mycode",
            "https://edgedl.me.gvt1.com/edgedl/chrome/dict/en-US-10-1.bdic",
            "https://challenges.cloudflare.com/turnstile/v0/api.js",
            "wss://remote.example.com/ws",
            "http://192.168.1.20:3773/api/health",
            "http://10.0.0.5:3773/",
            "http://0.0.0.0:3773/",
            "file:///C:/Windows/System32/cmd.exe",
            "not-a-url",
        ] {
            assert!(!is_allowed_offline_url(url), "expected blocked: {url}");
        }
    }

    #[test]
    fn loopback_http_url_rejects_non_http_and_remote() {
        assert!(is_loopback_http_url("http://127.0.0.1:18773/api/assets/1"));
        assert!(is_loopback_http_url("http://localhost:3773/img.png"));
        assert!(!is_loopback_http_url("https://example.com/img.png"));
        assert!(!is_loopback_http_url("ws://127.0.0.1:3773/ws"));
    }
}
