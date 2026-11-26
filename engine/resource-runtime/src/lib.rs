// SPDX-License-Identifier: MPL-2.0
//! Read package-local files relative to their referencing document and cache shared bytes.
pub mod json_integer;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Per-load resource snapshot. Paths are portable slash-separated package paths.
/// A cached file is immutable until the caller explicitly clears or drops this store.
#[derive(Debug)]
pub struct ResourceStore {
    root: PathBuf,
    files: HashMap<PathBuf, Arc<[u8]>>,
    aliases: HashMap<(PathBuf, PathBuf), PathBuf>,
}

fn relative(path: &str) -> Result<&Path, String> {
    if path.contains(['\\', ':'])
        || path.bytes().any(|b| b < 32)
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        Err(format!("invalid package-relative path {path:?}"))
    } else {
        Ok(Path::new(path))
    }
}

impl ResourceStore {
    /// Open an existing package directory. No file reads occur until requested.
    pub fn open(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|e| format!("{}: {e}", root.display()))?;
        if !root.is_dir() {
            return Err(format!("{}: expected package directory", root.display()));
        }
        Ok(Self {
            root,
            files: HashMap::new(),
            aliases: HashMap::new(),
        })
    }

    /// Read a regular file relative to the referencing document's parent directory.
    /// References must stay in that directory, including when resolving symlinks.
    pub fn read(&mut self, owner: &str, reference: &str) -> Result<Arc<[u8]>, String> {
        let parent = relative(owner)?
            .parent()
            .expect("validated relative filename");
        let logical = (parent.to_owned(), relative(reference)?.to_owned());
        if let Some(key) = self.aliases.get(&logical)
            && let Some(bytes) = self.files.get(key)
        {
            return Ok(Arc::clone(bytes));
        }
        let base = self
            .root
            .join(parent)
            .canonicalize()
            .map_err(|e| format!("{owner}: {e}"))?;
        if !base.starts_with(&self.root) {
            return Err(format!("{owner}: document directory escapes package"));
        }
        let path = base
            .join(reference)
            .canonicalize()
            .map_err(|e| format!("{owner} -> {reference}: {e}"))?;
        if !path.starts_with(&base) || !path.is_file() {
            return Err(format!(
                "{owner} -> {reference}: expected a regular file inside the document directory"
            ));
        }
        let bytes = match self.files.get(&path) {
            Some(bytes) => Arc::clone(bytes),
            None => {
                let bytes: Arc<[u8]> = fs::read(&path)
                    .map_err(|e| format!("{owner} -> {reference}: {e}"))?
                    .into();
                self.files.insert(path.clone(), Arc::clone(&bytes));
                bytes
            }
        };
        self.aliases.insert(logical, path);
        Ok(bytes)
    }

    /// Release all cached paths and owned byte buffers. Outstanding readers keep their Arc.
    pub fn clear(&mut self) {
        self.aliases.clear();
        self.files.clear();
    }
    /// Number of distinct resolved files currently held.
    pub fn cached_count(&self) -> usize {
        self.files.len()
    }
}
