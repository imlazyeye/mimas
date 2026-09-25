use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use api::{ApiEntry, Library, ManifestRef};
use shared::Literal;

/// Writes `library` to `path` as a manifest unless its output would be identical to what is already
/// present.
pub fn write_api(library: &Library<()>, path: &Path) -> std::io::Result<()> {
    // serde_json writes these as `null`, which can't be read back
    for (_, entry) in library.natives() {
        if let ApiEntry::Constant(constant) = entry
            && let Literal::Float(value) = constant.value
            && !value.is_finite()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("constant `{}` isn't a finite float", constant.name),
            ));
        }
    }

    let output = serde_json::to_vec_pretty(&ManifestRef {
        version: api::VERSION,
        library,
    })
    .expect("failed to serialize the manifest!");

    // No need to write this if it's the same as last time
    if let Ok(data) = std::fs::read(path)
        && data == output
    {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, output)
}

/// Where a host built as `exe` writes its manifest ([`api::manifest_file`] in its target dir).
/// `None` wherever [`target_dir`] is.
pub fn manifest_path(exe: &Path) -> Option<PathBuf> {
    let name = exe.file_stem()?.to_string_lossy();
    Some(api::manifest_file(target_dir(exe)?, &name))
}

/// The cargo target dir `exe` was built into, found by the `CACHEDIR.TAG` cargo writes at its
/// root. `None` for test and bench binaries and for anything outside a target dir.
pub fn target_dir(exe: &Path) -> Option<&Path> {
    let dir = exe.parent()?;
    if dir.file_name()? == "deps" {
        return None;
    }
    dir.ancestors()
        .find(|dir| dir.join("CACHEDIR.TAG").is_file())
}

pub(crate) fn auto_export(library: &Library<()>) {
    static WRITTEN: AtomicBool = AtomicBool::new(false);
    if WRITTEN.swap(true, Ordering::Relaxed) {
        return;
    }

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(path) = manifest_path(&exe) else {
        return;
    };
    if let Err(e) = write_api(library, &path) {
        eprintln!(
            "mimas: couldn't write the API manifest to {}: {e}",
            path.display()
        );
    }
}
