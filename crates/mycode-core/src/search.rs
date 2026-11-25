use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntryItem {
    pub path: String,
    pub kind: String, // "file" | "directory"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListResult {
    pub entries: Vec<WorkspaceEntryItem>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchMatch {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub line_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchResult {
    pub matches: Vec<ContentSearchMatch>,
    pub truncated: bool,
    pub files_scanned: u32,
}

fn to_posix_relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        parts.push(component.as_os_str().to_string_lossy());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Fast `.gitignore`-aware workspace file & directory lister.
pub fn list_workspace_entries(
    root: &Path,
    max_entries: usize,
    include_hidden: bool,
) -> std::io::Result<WorkspaceListResult> {
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Workspace root is not a directory: {}", root.display()),
        ));
    }

    let cap = max_entries.max(1);
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(!include_hidden)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| entry.file_name() != ".git");

    let mut entries = Vec::with_capacity(cap.min(4096));
    let mut truncated = false;

    for result in builder.build() {
        let Ok(entry) = result else { continue };
        if entry.depth() == 0 {
            continue;
        }
        let Some(rel_path) = to_posix_relative(root, entry.path()) else {
            continue;
        };
        let kind = if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            "directory"
        } else {
            "file"
        };
        if entries.len() >= cap {
            truncated = true;
            break;
        }
        entries.push(WorkspaceEntryItem {
            path: rel_path,
            kind: kind.to_string(),
        });
    }

    entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(WorkspaceListResult { entries, truncated })
}

/// Parallel `.gitignore`-aware substring content search across workspace files.
pub fn search_workspace_contents(
    root: &Path,
    query: &str,
    case_sensitive: bool,
    max_matches: usize,
    max_matches_per_file: usize,
) -> std::io::Result<ContentSearchResult> {
    if query.is_empty() {
        return Ok(ContentSearchResult {
            matches: Vec::new(),
            truncated: false,
            files_scanned: 0,
        });
    }

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| entry.file_name() != ".git");

    let files: Vec<PathBuf> = builder
        .build()
        .filter_map(|res| res.ok())
        .filter(|e| e.depth() > 0 && e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
        .collect();

    let files_scanned = files.len() as u32;
    let needle = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };
    let per_file_cap = max_matches_per_file.max(1);
    let total_cap = max_matches.max(1);

    let mut all_matches: Vec<ContentSearchMatch> = files
        .par_iter()
        .flat_map_iter(|path| {
            let Ok(meta) = std::fs::metadata(path) else {
                return Vec::new();
            };
            // Skip oversized/binary files (> 4 MiB).
            if meta.len() > 4 * 1024 * 1024 {
                return Vec::new();
            }
            let Ok(file) = File::open(path) else {
                return Vec::new();
            };
            let Some(rel_path) = to_posix_relative(root, path) else {
                return Vec::new();
            };

            let mut reader = BufReader::new(file);
            let mut buf = Vec::with_capacity(512);
            let mut line_num: u32 = 0;
            let mut file_matches = Vec::new();

            loop {
                buf.clear();
                let Ok(read) = reader.read_until(b'\n', &mut buf) else {
                    break;
                };
                if read == 0 {
                    break;
                }
                // Treat NUL bytes in early lines as binary and bail out.
                if line_num < 8 && buf.contains(&0) {
                    return Vec::new();
                }
                line_num += 1;
                let line_str = String::from_utf8_lossy(&buf);
                let trimmed = line_str.trim_end_matches(&['\r', '\n'][..]);
                let found_col = if case_sensitive {
                    trimmed.find(&needle)
                } else {
                    trimmed.to_lowercase().find(&needle)
                };
                if let Some(byte_idx) = found_col {
                    let preview: String = trimmed.chars().take(512).collect();
                    file_matches.push(ContentSearchMatch {
                        path: rel_path.clone(),
                        line: line_num,
                        column: (byte_idx + 1) as u32,
                        line_text: preview,
                    });
                    if file_matches.len() >= per_file_cap {
                        break;
                    }
                }
            }
            file_matches
        })
        .collect();

    all_matches.sort_unstable_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    let truncated = all_matches.len() > total_cap;
    if truncated {
        all_matches.truncate(total_cap);
    }

    Ok(ContentSearchResult {
        matches: all_matches,
        truncated,
        files_scanned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_and_searches_respecting_gitignore() {
        let dir = std::env::temp_dir().join(format!("mycode-search-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("ignored_dir")).unwrap();
        fs::write(dir.join(".gitignore"), "ignored_dir/\n").unwrap();
        fs::write(dir.join("src/hello.rs"), "fn main() {\n    println!(\"MyCode Rust\");\n}\n")
            .unwrap();
        fs::write(dir.join("ignored_dir/secret.txt"), "MyCode Rust hidden\n").unwrap();

        let listed = list_workspace_entries(&dir, 100, false).unwrap();
        assert!(listed.entries.iter().any(|e| e.path == "src/hello.rs"));
        assert!(!listed.entries.iter().any(|e| e.path.contains("ignored_dir")));

        let searched = search_workspace_contents(&dir, "mycode rust", false, 50, 10).unwrap();
        assert_eq!(searched.matches.len(), 1);
        assert_eq!(searched.matches[0].path, "src/hello.rs");
        assert_eq!(searched.matches[0].line, 2);

        let _ = fs::remove_dir_all(&dir);
    }
}
