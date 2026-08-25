use crate::error::{Error, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The host surface [`write_tree_atomic`] needs: the rendered asset tree
/// (`walk`) plus the manifest files (`manifest_files`).
pub trait TreeWriter {
    /// Invoke `f` once per rendered asset with its slash-relative path and
    /// bytes. A non-`Ok` return aborts the walk.
    fn walk(&self, f: &mut dyn FnMut(&str, &[u8]) -> Result<()>) -> Result<()>;

    /// The plugin manifests files keyed by slash-relative path (e.g. `.mcp.json`).
    fn manifest_files(&self) -> Result<Vec<(String, Vec<u8>)>>;
}

/// Map a forward-slash relative path to the OS-native form (Rust
/// `filepath.FromSlash`).
pub(crate) fn from_slash(rel: &str) -> PathBuf {
    if std::path::MAIN_SEPARATOR == '/' {
        PathBuf::from(rel)
    } else {
        PathBuf::from(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
    }
}

/// Write every asset + manifest file from `h` into `target` using tmp+rename,
/// then sweep stale files under `sweep_dirs`. Returns the number of files
/// written.
pub fn write_tree_atomic(h: &dyn TreeWriter, target: &str, sweep_dirs: &[&str]) -> Result<i32> {
    let target = Path::new(target);
    std::fs::create_dir_all(target)
        .map_err(|e| io_err(format!("mkdir {}", target.display()), e))?;

    let mut wanted: HashSet<PathBuf> = HashSet::new();
    let mut count: i32 = 0;

    let write_one =
        |wanted: &mut HashSet<PathBuf>, count: &mut i32, rel: &str, data: &[u8]| -> Result<()> {
            let dst = target.join(from_slash(rel));
            wanted.insert(dst.clone());
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| io_err(format!("mkdir parent of {rel}"), e))?;
            }
            atomic_write_file(&dst, data).map_err(|e| io_err(format!("write {rel}"), e))?;
            *count += 1;
            Ok(())
        };

    // Walk rendered assets.
    {
        let mut cb = |rel: &str, data: &[u8]| write_one(&mut wanted, &mut count, rel, data);
        h.walk(&mut cb)?;
    }

    // Manifest files.
    let mfs = h.manifest_files()?;
    for (rel, data) in &mfs {
        write_one(&mut wanted, &mut count, rel, data)?;
    }

    // Sweep stale files under each sweep dir.
    for sub in sweep_dirs {
        let sub_path = target.join(sub);
        sweep(&sub_path, &wanted);
    }

    Ok(count)
}

/// Recursively remove any regular file under `dir` not present in `wanted`
/// (Rust's `filepath.WalkDir` + `os.Remove`). Walk errors are ignored, matching
/// the source implementation's `_ = filepath.WalkDir(...)`.
fn sweep(dir: &Path, wanted: &HashSet<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => sweep(&p, wanted),
            Ok(_) => {
                if !wanted.contains(&p) && std::fs::remove_file(&p).is_ok() {
                    eprintln!("[install] swept stale: {}", p.display());
                }
            }
            Err(_) => {}
        }
    }
}

pub(crate) fn atomic_write_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// Map an `io::Error` into the aihost error with a `aihost:`-style prefix.
pub(crate) fn io_err(context: impl std::fmt::Display, e: impl std::fmt::Display) -> Error {
    Error::Io(format!("aihost: {context}: {e}"))
}
