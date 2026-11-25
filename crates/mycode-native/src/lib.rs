use mycode_core::{firewall, git, pty, search, transcript};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

fn global_pty() -> &'static pty::PtyManager {
    static PTY: OnceLock<pty::PtyManager> = OnceLock::new();
    PTY.get_or_init(pty::PtyManager::new)
}

#[napi(object)]
pub struct NativeTranscriptFile {
    pub path: String,
    pub size: f64,
    pub mtime_ms: f64,
}

#[napi(object)]
pub struct NativeTranscriptWalkResult {
    pub files: Vec<NativeTranscriptFile>,
    pub walked: Vec<String>,
    pub failed_paths: u32,
}

#[napi(object)]
pub struct NativeWorkspaceEntry {
    pub path: String,
    pub kind: String,
}

#[napi(object)]
pub struct NativeWorkspaceListResult {
    pub entries: Vec<NativeWorkspaceEntry>,
    pub truncated: bool,
}

#[napi(object)]
pub struct NativeContentSearchMatch {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub line_text: String,
}

#[napi(object)]
pub struct NativeContentSearchResult {
    pub matches: Vec<NativeContentSearchMatch>,
    pub truncated: bool,
    pub files_scanned: u32,
}

#[napi(object)]
pub struct NativeGitRepoSummary {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged_count: u32,
    pub unstaged_count: u32,
    pub untracked_count: u32,
    pub has_working_tree_changes: bool,
}

#[napi(object)]
pub struct NativePtySpawnOptions {
    pub shell: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub cols: u32,
    pub rows: u32,
    pub env: Option<HashMap<String, String>>,
}

#[napi(object)]
pub struct NativePtySpawnInfo {
    pub session_id: String,
    pub pid: Option<u32>,
}

#[napi(object)]
pub struct NativePtyPollResult {
    pub data: Buffer,
    pub exited: bool,
    pub exit_code: Option<u32>,
}

/// Fast 32-bit FNV-1a guard hash matching `usageTranscriptReader.ts`.
#[napi(js_name = "fnv1aHash")]
pub fn fnv1a_hash(buf: Buffer) -> u32 {
    transcript::fnv1a_32(buf.as_ref())
}

/// Parallel Rust directory walker + mtime filter for usage transcripts (`.jsonl` / `.jsonl.zst`).
#[napi]
pub fn list_transcript_files(
    root: String,
    since_ms: f64,
    file_name: Option<String>,
) -> NativeTranscriptWalkResult {
    let result = transcript::list_transcript_files(Path::new(&root), since_ms, file_name.as_deref());
    NativeTranscriptWalkResult {
        files: result
            .files
            .into_iter()
            .map(|f| NativeTranscriptFile {
                path: f.path,
                size: f.size,
                mtime_ms: f.mtime_ms,
            })
            .collect(),
        walked: result.walked,
        failed_paths: result.failed_paths,
    }
}

/// Fast `.gitignore`-aware workspace file & directory listing.
#[napi]
pub fn list_workspace_entries(
    root: String,
    max_entries: u32,
    include_hidden: bool,
) -> Result<NativeWorkspaceListResult> {
    let res = search::list_workspace_entries(Path::new(&root), max_entries as usize, include_hidden)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(NativeWorkspaceListResult {
        entries: res
            .entries
            .into_iter()
            .map(|e| NativeWorkspaceEntry {
                path: e.path,
                kind: e.kind,
            })
            .collect(),
        truncated: res.truncated,
    })
}

/// Parallel `.gitignore`-aware substring content search across workspace files.
#[napi]
pub fn search_workspace_contents(
    root: String,
    query: String,
    case_sensitive: bool,
    max_matches: u32,
    max_matches_per_file: u32,
) -> Result<NativeContentSearchResult> {
    let res = search::search_workspace_contents(
        Path::new(&root),
        &query,
        case_sensitive,
        max_matches as usize,
        max_matches_per_file as usize,
    )
    .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(NativeContentSearchResult {
        matches: res
            .matches
            .into_iter()
            .map(|m| NativeContentSearchMatch {
                path: m.path,
                line: m.line,
                column: m.column,
                line_text: m.line_text,
            })
            .collect(),
        truncated: res.truncated,
        files_scanned: res.files_scanned,
    })
}

/// Local git repository working-tree summary (`git status --porcelain=v2 --branch`).
#[napi]
pub fn git_repo_summary(cwd: String) -> NativeGitRepoSummary {
    let s = git::git_repo_summary(Path::new(&cwd));
    NativeGitRepoSummary {
        is_repo: s.is_repo,
        branch: s.branch,
        head_oid: s.head_oid,
        upstream: s.upstream,
        ahead: s.ahead,
        behind: s.behind,
        staged_count: s.staged_count,
        unstaged_count: s.unstaged_count,
        untracked_count: s.untracked_count,
        has_working_tree_changes: s.has_working_tree_changes,
    }
}

/// Air-gapped loopback URL validation matching the desktop WebView firewall.
#[napi]
pub fn is_allowed_offline_url(url: String) -> bool {
    firewall::is_allowed_offline_url(&url)
}

#[napi]
pub fn pty_spawn(options: NativePtySpawnOptions) -> Result<NativePtySpawnInfo> {
    let info = global_pty()
        .spawn(pty::PtySpawnRequest {
            shell: options.shell,
            args: options.args,
            cwd: options.cwd,
            cols: options.cols.min(u16::MAX as u32) as u16,
            rows: options.rows.min(u16::MAX as u32) as u16,
            env: options.env.unwrap_or_default(),
        })
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(NativePtySpawnInfo {
        session_id: info.session_id,
        pid: info.pid,
    })
}

#[napi]
pub fn pty_write(session_id: String, data: Buffer) -> Result<()> {
    global_pty()
        .write(&session_id, data.as_ref())
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn pty_resize(session_id: String, cols: u32, rows: u32) -> Result<()> {
    global_pty()
        .resize(
            &session_id,
            cols.min(u16::MAX as u32) as u16,
            rows.min(u16::MAX as u32) as u16,
        )
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn pty_poll(session_id: String) -> Result<NativePtyPollResult> {
    let res = global_pty()
        .poll(&session_id)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(NativePtyPollResult {
        data: Buffer::from(res.data),
        exited: res.exited,
        exit_code: res.exit_code,
    })
}

#[napi]
pub fn pty_kill(session_id: String) -> Result<()> {
    global_pty()
        .kill(&session_id)
        .map_err(|e| Error::from_reason(e.to_string()))
}
