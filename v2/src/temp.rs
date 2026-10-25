//! Each artifact owns a private 0700 directory and a 0600 exclusive file. No
//! global source/session map. Persistence is checkpointed synchronously before
//! any await; a dropped worker result owns RAII cleanup automatically.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub directory: PathBuf,
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: usize,
    pub directory_id: String,
    pub file_id: String,
}
#[derive(Debug)]
pub struct Pending {
    armed: bool,
    pub artifact: Artifact,
    pub retired_staging_path: PathBuf,
}
impl Pending {
    pub fn commit(mut self) -> Artifact {
        // No await between detaching RAII ownership and writing session state.
        self.armed = false;
        self.artifact.clone()
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        if self.armed {
            let _ = cleanup(&self.artifact);
        }
    }
}
pub fn create(base: &Path, content: &str, sample: bool) -> Result<Pending, &'static str> {
    if content.len() > crate::MAX_OUTPUT_BYTES {
        return Err("Browser document exceeds the 8 MiB resource limit");
    }
    // Caller base must already exist. Never create arbitrary user parent trees.
    let mut builder = tempfile::Builder::new();
    builder.prefix("tezbar-markdown-styler-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    let directory = builder
        .tempdir_in(base)
        .map_err(|_| "Unable to create temporary document")?;
    let canonical = directory
        .path()
        .canonicalize()
        .map_err(|_| "Unable to resolve temporary document")?;
    let filename = if sample {
        "md-styler-sample.html"
    } else {
        "md-styler.html"
    };
    let path = canonical.join(filename);
    // Persist_noclobber disarms NamedTempFile's staging-path cleanup after success.
    let mut file = tempfile::Builder::new()
        .prefix(".stage-")
        .tempfile_in(&canonical)
        .map_err(|_| "Unable to create temporary document")?;
    file.write_all(content.as_bytes())
        .map_err(|_| "Unable to write temporary document")?;
    file.flush()
        .map_err(|_| "Unable to flush temporary document")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Unable to sync temporary document")?;
    let retired_staging_path = file.path().to_owned();
    file.persist_noclobber(&path)
        .map_err(|_| "Unable to publish temporary document")?;
    let directory_id =
        file_id::get_file_id(&canonical).map_err(|_| "Unable to identify temporary directory")?;
    let file_id =
        file_id::get_file_id(&path).map_err(|_| "Unable to identify temporary document")?;
    let artifact = Artifact {
        directory_id: format!("{directory_id:?}"),
        file_id: format!("{file_id:?}"),
        directory: canonical,
        path,
        sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
        bytes: content.len(),
    };
    let _ = directory.keep();
    Ok(Pending {
        armed: true,
        artifact,
        retired_staging_path,
    })
}
fn shape(a: &Artifact) -> bool {
    a.directory
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.starts_with("tezbar-markdown-styler-"))
        && a.path.parent() == Some(a.directory.as_path())
        && matches!(
            a.path.file_name().and_then(|s| s.to_str()),
            Some("md-styler.html" | "md-styler-sample.html")
        )
}
pub fn verify(a: &Artifact) -> Result<(), &'static str> {
    if !shape(a) {
        return Err("Temporary document ownership is invalid");
    }
    let dir = fs::symlink_metadata(&a.directory).map_err(|_| "Temporary file does not exist")?;
    let meta = fs::symlink_metadata(&a.path).map_err(|_| "Temporary file does not exist")?;
    if !dir.is_dir()
        || dir.file_type().is_symlink()
        || !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.len() != a.bytes as u64
        || a.bytes > crate::MAX_OUTPUT_BYTES
    {
        return Err("Temporary document was replaced");
    }
    if file_id::get_file_id(&a.directory)
        .map(|id| format!("{id:?}"))
        .ok()
        .as_ref()
        != Some(&a.directory_id)
        || file_id::get_file_id(&a.path)
            .map(|id| format!("{id:?}"))
            .ok()
            .as_ref()
            != Some(&a.file_id)
    {
        return Err("Temporary document identity changed");
    }
    if a.directory.canonicalize().ok().as_ref() != Some(&a.directory) {
        return Err("Temporary document was replaced");
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(&a.path)
        .map_err(|_| "Unable to read temporary document")?
        .take((crate::MAX_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Unable to read temporary document")?;
    if format!("{:x}", Sha256::digest(&bytes)) != a.sha256 {
        return Err("Temporary document content changed");
    }
    Ok(())
}
pub fn cleanup(a: &Artifact) -> Result<(), &'static str> {
    // Delete only a matching owned artifact; never recurse. Changed content,
    // symlinks and unrelated entries survive and surface a cleanup error.
    if !shape(a) {
        return Err("Temporary document ownership is invalid");
    }
    let dir = match fs::symlink_metadata(&a.directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Unable to identify owned temporary directory"),
        Ok(m) => m,
    };
    if !dir.is_dir()
        || dir.file_type().is_symlink()
        || a.directory.canonicalize().ok().as_ref() != Some(&a.directory)
        || file_id::get_file_id(&a.directory)
            .map(|id| format!("{id:?}"))
            .ok()
            .as_ref()
            != Some(&a.directory_id)
    {
        return Err("Temporary directory identity changed");
    }
    if a.path.exists() || fs::symlink_metadata(&a.path).is_ok() {
        verify(a)?;
        fs::remove_file(&a.path).map_err(|_| "Unable to remove owned temporary file")?;
    }
    fs::remove_dir(&a.directory)
        .map_err(|_| "Temporary directory contains unowned entries or cannot be removed")
}
pub fn file_url(a: &Artifact) -> Result<String, &'static str> {
    verify(a)?;
    url::Url::from_file_path(&a.path)
        .map(|u| u.to_string())
        .map_err(|_| "Unable to encode temporary document URL")
}
