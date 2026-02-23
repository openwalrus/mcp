//! Path validation and security for the filesystem MCP server.
//!
//! All filesystem operations must pass through [`validate_path`] to ensure
//! the requested path is within the server's allowed directories.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors from path validation.
#[derive(Error, Debug)]
pub enum ValidateError {
    /// The path is outside all allowed directories.
    #[error("path not allowed: {0}")]
    NotAllowed(PathBuf),
    /// The path contains a null byte.
    #[error("path contains null byte")]
    NullByte,
    /// An I/O error occurred during path resolution.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Validate that a path is within the allowed directories.
///
/// Steps:
/// 1. Reject paths containing null bytes
/// 2. Canonicalize the path (resolves symlinks, `..`, etc.)
///    - If the path does not exist, canonicalize the parent directory instead
/// 3. Verify the canonical path starts with one of the allowed directories
pub fn validate_path(path: &str, allowed_dirs: &[PathBuf]) -> Result<PathBuf, ValidateError> {
    if path.contains('\0') {
        return Err(ValidateError::NullByte);
    }

    let path = Path::new(path);

    let canonical = if path.exists() {
        path.canonicalize()?
    } else {
        let parent = path.parent().ok_or_else(|| {
            ValidateError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "parent directory not found",
            ))
        })?;
        let canon_parent = parent.canonicalize()?;
        let file_name = path.file_name().ok_or_else(|| {
            ValidateError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no file name",
            ))
        })?;
        canon_parent.join(file_name)
    };

    let allowed = allowed_dirs.iter().any(|dir| canonical.starts_with(dir));
    if !allowed {
        return Err(ValidateError::NotAllowed(canonical));
    }

    Ok(canonical)
}

/// Canonicalize a list of directory paths, skipping any that don't exist.
pub fn canonicalize_dirs(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    dirs.into_iter()
        .filter_map(|d| d.canonicalize().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use crate::validate::{canonicalize_dirs, validate_path};

    #[test]
    fn allows_path_within_dir() {
        let tmp = std::env::temp_dir();
        let allowed = canonicalize_dirs(vec![tmp.clone()]);
        let test_path = tmp.join("wmcp_test_validate.txt");
        fs::write(&test_path, "test").unwrap();
        let result = validate_path(test_path.to_str().unwrap(), &allowed);
        assert!(result.is_ok());
        fs::remove_file(&test_path).ok();
    }

    #[test]
    fn rejects_path_outside_dir() {
        let allowed = canonicalize_dirs(vec!["/tmp/wmcp_nonexistent_dir_xyz".into()]);
        let result = validate_path("/etc/passwd", &allowed);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_null_byte() {
        let allowed = canonicalize_dirs(vec![std::env::temp_dir()]);
        let result = validate_path("/tmp/foo\0bar", &allowed);
        assert!(result.is_err());
    }

    #[test]
    fn allows_nonexistent_file_in_allowed_dir() {
        let tmp = std::env::temp_dir();
        let allowed = canonicalize_dirs(vec![tmp.clone()]);
        let path = tmp.join("wmcp_nonexistent_file_test.txt");
        let result = validate_path(path.to_str().unwrap(), &allowed);
        assert!(result.is_ok());
    }
}
