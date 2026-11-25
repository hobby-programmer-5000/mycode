use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptFileEntry {
    pub path: String,
    pub size: f64,
    pub mtime_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptWalkResult {
    pub files: Vec<TranscriptFileEntry>,
    pub walked: Vec<String>,
    pub failed_paths: u32,
}

/// 32-bit FNV-1a hash matching `apps/server/src/usage/usageTranscriptReader.ts`.
#[inline]
pub fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn walk_collect(
    dir: &Path,
    exact_file_name: Option<&str>,
    failed_paths: &AtomicU32,
) -> Vec<PathBuf> {
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => {
            failed_paths.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
    };

    let mut subdirs = Vec::new();
    let mut matched = Vec::new();

    for entry_res in entries {
        let Ok(entry) = entry_res else {
            failed_paths.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let Ok(ft) = entry.file_type() else {
            failed_paths.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let path = entry.path();
        if ft.is_dir() {
            subdirs.push(path);
        } else if ft.is_file() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let is_match = match exact_file_name {
                Some(target) => name_str == target,
                None => name_str.ends_with(".jsonl") || name_str.ends_with(".jsonl.zst"),
            };
            if is_match {
                matched.push(path);
            }
        }
    }

    if !subdirs.is_empty() {
        let nested: Vec<Vec<PathBuf>> = subdirs
            .par_iter()
            .map(|sub| walk_collect(sub, exact_file_name, failed_paths))
            .collect();
        for batch in nested {
            matched.extend(batch);
        }
    }

    matched
}

/// Parallel transcript directory walker + mtime filter replacing slow JS `readdir`/`stat` loops.
pub fn list_transcript_files(
    root: &Path,
    since_ms: f64,
    file_name: Option<&str>,
) -> TranscriptWalkResult {
    let failed_paths = AtomicU32::new(0);
    let candidates = walk_collect(root, file_name, &failed_paths);

    let walked: Vec<String> = candidates
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let files: Vec<TranscriptFileEntry> = candidates
        .par_iter()
        .filter_map(|path| match fs::metadata(path) {
            Ok(meta) => {
                let mtime_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .unwrap_or(0.0);
                if mtime_ms >= since_ms {
                    Some(TranscriptFileEntry {
                        path: path.to_string_lossy().into_owned(),
                        size: meta.len() as f64,
                        mtime_ms,
                    })
                } else {
                    None
                }
            }
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    failed_paths.fetch_add(1, Ordering::Relaxed);
                }
                None
            }
        })
        .collect();

    TranscriptWalkResult {
        files,
        walked,
        failed_paths: failed_paths.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_standard_vectors() {
        assert_eq!(fnv1a_32(b""), 0x811c_9dc5);
        assert_eq!(fnv1a_32(b"hello"), 0x4f9f_2cab);
        assert_eq!(fnv1a_32(b"mycode"), 0xcffd_e70a);
    }

    #[test]
    fn walks_and_filters_jsonl_files() {
        let dir =
            std::env::temp_dir().join(format!("mycode-transcript-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.jsonl"), "{\"ok\":true}\n").unwrap();
        fs::write(dir.join("sub/b.jsonl.zst"), b"compressed").unwrap();
        fs::write(dir.join("ignore.txt"), "nope").unwrap();

        let res = list_transcript_files(&dir, 0.0, None);
        assert_eq!(res.failed_paths, 0);
        assert_eq!(res.walked.len(), 2);
        assert_eq!(res.files.len(), 2);

        let exact = list_transcript_files(&dir, 0.0, Some("a.jsonl"));
        assert_eq!(exact.files.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
