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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Package(PathBuf);
    impl Package {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ofe-resource-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir(path.join("animations")).unwrap();
            fs::write(path.join("animations/image"), b"first").unwrap();
            Self(path)
        }
    }
    impl Drop for Package {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn cache_identity_snapshot_and_reload() {
        let package = Package::new();
        let mut store = ResourceStore::open(&package.0).unwrap();
        let handle = store.read("manifest.json", "animations/image").unwrap();
        let alias = store.read("animations/idle.json", "image").unwrap();
        assert!(Arc::ptr_eq(&handle, &alias));
        assert_eq!(store.cached_count(), 1);
        fs::write(package.0.join("animations/image"), b"second").unwrap();
        assert!(Arc::ptr_eq(
            &handle,
            &store.read("manifest.json", "animations/image").unwrap()
        ));
        store.clear();
        assert_eq!(store.cached_count(), 0);
        let updated = store.read("animations/idle.json", "image").unwrap();
        drop(store);
        assert_eq!(&*handle, b"first");
        assert_eq!(&*updated, b"second");
    }

    #[test]
    fn reject_nonfiles_and_invalid_paths_for_both_arguments() {
        let package = Package::new();
        assert!(ResourceStore::open(&package.0.join("animations/image")).is_err());
        let mut store = ResourceStore::open(&package.0).unwrap();
        for bad in [
            "",
            "/etc/passwd",
            "../image",
            "sub/../image",
            "./image",
            "sub//image",
            "C:/image",
            "a:b",
            "a\\b",
            "a\u{0}b",
            "dir/",
        ] {
            assert!(store.read("manifest.json", bad).is_err(), "{bad:?}");
            assert!(store.read(bad, "animations/image").is_err(), "{bad:?}");
        }
        assert!(store.read("manifest.json", "animations").is_err());
        assert!(store.read("manifest.json", "missing.png").is_err());
        assert_eq!(store.cached_count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn scoped_resolution_applies_even_after_root_cached_an_alias() {
        use std::os::unix::fs::symlink;
        let package = Package::new();
        fs::write(package.0.join("root-image"), b"root").unwrap();
        symlink("../root-image", package.0.join("animations/link")).unwrap();
        symlink("image", package.0.join("animations/local-link")).unwrap();
        let mut store = ResourceStore::open(&package.0).unwrap();
        assert_eq!(
            &*store.read("manifest.json", "animations/link").unwrap(),
            b"root"
        );
        assert!(store.read("animations/idle.json", "link").is_err());
        let image = store.read("animations/idle.json", "image").unwrap();
        assert!(Arc::ptr_eq(
            &image,
            &store.read("animations/idle.json", "local-link").unwrap()
        ));
        assert_eq!(store.cached_count(), 2);

        let other = Package::new();
        symlink(&other.0, package.0.join("foreign")).unwrap();
        assert!(store.read("foreign/idle.json", "animations/image").is_err());
        assert!(
            store
                .read("manifest.json", "foreign/animations/image")
                .is_err()
        );
    }
}
