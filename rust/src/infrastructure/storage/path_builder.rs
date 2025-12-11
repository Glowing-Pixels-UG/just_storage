use std::path::{Path, PathBuf};

use crate::domain::value_objects::{ContentHash, StorageClass};

/// Utility for generating storage paths
pub struct PathBuilder {
    hot_root: PathBuf,
    cold_root: PathBuf,
}

impl PathBuilder {
    pub fn new(hot_root: PathBuf, cold_root: PathBuf) -> Self {
        Self {
            hot_root,
            cold_root,
        }
    }

    /// Get root path for storage class
    fn root(&self, storage_class: StorageClass) -> &Path {
        match storage_class {
            StorageClass::Hot => &self.hot_root,
            StorageClass::Cold => &self.cold_root,
        }
    }

    /// Generate temp upload path: /root/temp/{uuid}
    pub fn temp_path(&self, storage_class: StorageClass, id: uuid::Uuid) -> PathBuf {
        self.root(storage_class).join("temp").join(id.to_string())
    }

    /// Generate final content-addressable path: /root/sha256/{prefix}/{hash}
    pub fn final_path(&self, storage_class: StorageClass, hash: &ContentHash) -> PathBuf {
        let prefix = hash.prefix();
        self.root(storage_class)
            .join("sha256")
            .join(prefix)
            .join(hash.as_hex())
    }
}
