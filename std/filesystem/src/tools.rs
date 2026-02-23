//! Tool implementations for the filesystem MCP server.

use std::future::Future;
use crate::FilesystemServer;
use crate::validate::validate_path;
use rmcp::{
    handler::server::wrapper::Parameters,
    schemars::{self, JsonSchema},
    tool, tool_router,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parameters for reading a single file.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file to read.
    pub path: String,
}

/// Parameters for reading multiple files.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadMultipleFilesParams {
    /// Paths to the files to read.
    pub paths: Vec<String>,
}

/// Parameters for writing a file.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteFileParams {
    /// Path to the file to write.
    pub path: String,
    /// Content to write to the file.
    pub content: String,
}

/// A single text edit operation.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditOperation {
    /// The text to search for.
    pub old_text: String,
    /// The replacement text.
    pub new_text: String,
}

/// Parameters for editing a file.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditFileParams {
    /// Path to the file to edit.
    pub path: String,
    /// List of edit operations to apply sequentially.
    pub edits: Vec<EditOperation>,
    /// If true, return the diff without writing changes.
    pub dry_run: Option<bool>,
}

/// Parameters for creating a directory.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateDirectoryParams {
    /// Path of the directory to create.
    pub path: String,
}

/// Parameters for listing a directory.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDirectoryParams {
    /// Path to the directory to list.
    pub path: String,
}

/// Parameters for getting a directory tree.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryTreeParams {
    /// Path to the root directory for the tree.
    pub path: String,
}

/// Parameters for moving a file or directory.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveFileParams {
    /// Source path.
    pub source: String,
    /// Destination path.
    pub destination: String,
}

/// Parameters for searching files.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchFilesParams {
    /// Base directory to search in.
    pub path: String,
    /// Glob pattern to match (e.g. "**/*.rs").
    pub pattern: String,
    /// Glob patterns to exclude from results.
    pub exclude_patterns: Option<Vec<String>>,
}

/// Parameters for getting file info.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFileInfoParams {
    /// Path to the file or directory.
    pub path: String,
}

/// File metadata returned by `get_file_info`.
#[derive(Debug, Serialize)]
struct FileInfo {
    size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    is_dir: bool,
    is_file: bool,
    is_symlink: bool,
    #[cfg(unix)]
    permissions: String,
}

/// A node in the directory tree.
#[derive(Debug, Serialize)]
struct TreeNode {
    name: String,
    #[serde(rename = "type")]
    node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TreeNode>>,
}

/// Result entry for reading multiple files.
#[derive(Debug, Serialize)]
struct FileReadResult {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tool_router]
impl FilesystemServer {
    /// Create a new filesystem server with the given allowed directories.
    pub fn new(allowed_dirs: Vec<std::path::PathBuf>) -> Self {
        let allowed_dirs = crate::validate::canonicalize_dirs(allowed_dirs);
        Self {
            allowed_dirs,
            tool_router: Self::tool_router(),
        }
    }

    /// Read the complete contents of a text file.
    #[tool(description = "Read the complete contents of a file from the filesystem")]
    async fn read_file(
        &self,
        Parameters(params): Parameters<ReadFileParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| e.to_string())
    }

    /// Read multiple files simultaneously.
    #[tool(
        description = "Read multiple files simultaneously, returning content or error for each file"
    )]
    async fn read_multiple_files(
        &self,
        Parameters(params): Parameters<ReadMultipleFilesParams>,
    ) -> Result<String, String> {
        let mut results = Vec::with_capacity(params.paths.len());
        for p in &params.paths {
            let entry = match validate_path(p, &self.allowed_dirs) {
                Ok(path) => match tokio::fs::read_to_string(&path).await {
                    Ok(content) => FileReadResult {
                        path: p.clone(),
                        content: Some(content),
                        error: None,
                    },
                    Err(e) => FileReadResult {
                        path: p.clone(),
                        content: None,
                        error: Some(e.to_string()),
                    },
                },
                Err(e) => FileReadResult {
                    path: p.clone(),
                    content: None,
                    error: Some(e.to_string()),
                },
            };
            results.push(entry);
        }
        serde_json::to_string_pretty(&results).map_err(|e| e.to_string())
    }

    /// Create or overwrite a file.
    #[tool(description = "Create a new file or overwrite an existing file with the given content")]
    async fn write_file(
        &self,
        Parameters(params): Parameters<WriteFileParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        tokio::fs::write(&path, &params.content)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("Successfully wrote to {}", path.display()))
    }

    /// Apply sequential text edits to a file.
    #[tool(
        description = "Make line-based edits to a file. Set dry_run to true to preview changes as a diff without writing"
    )]
    async fn edit_file(
        &self,
        Parameters(params): Parameters<EditFileParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let original = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| e.to_string())?;
        let mut content = original.clone();

        for edit in &params.edits {
            if !content.contains(&edit.old_text) {
                return Err(format!("Text not found in file: {:?}", edit.old_text));
            }
            content = content.replacen(&edit.old_text, &edit.new_text, 1);
        }

        // Build a simple unified diff
        let diff = build_diff(&original, &content);

        if params.dry_run.unwrap_or(false) {
            Ok(diff)
        } else {
            tokio::fs::write(&path, &content)
                .await
                .map_err(|e| e.to_string())?;
            Ok(diff)
        }
    }

    /// Create a directory and all parent directories.
    #[tool(
        description = "Create a new directory or ensure a directory exists, creating parent directories as needed"
    )]
    async fn create_directory(
        &self,
        Parameters(params): Parameters<CreateDirectoryParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("Successfully created directory {}", path.display()))
    }

    /// List files and directories in a path.
    #[tool(description = "List files and directories in a given path with type indicators")]
    async fn list_directory(
        &self,
        Parameters(params): Parameters<ListDirectoryParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&path)
            .await
            .map_err(|e| e.to_string())?;
        while let Some(entry) = read_dir.next_entry().await.map_err(|e| e.to_string())? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ft = entry.file_type().await.map_err(|e| e.to_string())?;
            if ft.is_dir() {
                entries.push(format!("{name}/"));
            } else {
                entries.push(name);
            }
        }
        entries.sort();
        Ok(entries.join("\n"))
    }

    /// Get a recursive tree view of files and directories.
    #[tool(description = "Get a recursive tree view of files and directories as JSON")]
    async fn directory_tree(
        &self,
        Parameters(params): Parameters<DirectoryTreeParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let tree = build_tree(&path).await.map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&tree).map_err(|e| e.to_string())
    }

    /// Move or rename a file or directory.
    #[tool(description = "Move or rename a file or directory")]
    async fn move_file(
        &self,
        Parameters(params): Parameters<MoveFileParams>,
    ) -> Result<String, String> {
        let source =
            validate_path(&params.source, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let dest =
            validate_path(&params.destination, &self.allowed_dirs).map_err(|e| e.to_string())?;
        tokio::fs::rename(&source, &dest)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "Moved {} to {}",
            source.display(),
            dest.display()
        ))
    }

    /// Search for files matching a glob pattern.
    #[tool(description = "Search for files matching a glob pattern within a directory")]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<String, String> {
        let base = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let full_pattern = base.join(&params.pattern);
        let pattern_str = full_pattern.to_string_lossy();
        let matches: Vec<String> = glob::glob(&pattern_str)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .filter(|path| {
                // Only include results within allowed directories
                self.allowed_dirs.iter().any(|dir| path.starts_with(dir))
            })
            .filter(|path| {
                // Apply exclude patterns if any
                if let Some(excludes) = &params.exclude_patterns {
                    let path_str = path.to_string_lossy();
                    !excludes.iter().any(|ex| {
                        glob::Pattern::new(ex)
                            .map(|p| p.matches(&path_str))
                            .unwrap_or(false)
                    })
                } else {
                    true
                }
            })
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        Ok(matches.join("\n"))
    }

    /// Get detailed metadata about a file or directory.
    #[tool(description = "Get detailed metadata about a file or directory")]
    async fn get_file_info(
        &self,
        Parameters(params): Parameters<GetFileInfoParams>,
    ) -> Result<String, String> {
        let path = validate_path(&params.path, &self.allowed_dirs).map_err(|e| e.to_string())?;
        let meta = tokio::fs::symlink_metadata(&path)
            .await
            .map_err(|e| e.to_string())?;

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()))
            .map(|dt| dt.to_rfc3339());
        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()))
            .map(|dt| dt.to_rfc3339());

        let info = FileInfo {
            size: meta.len(),
            modified,
            created,
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            is_symlink: meta.is_symlink(),
            #[cfg(unix)]
            permissions: {
                use std::os::unix::fs::PermissionsExt;
                format!("{:o}", meta.permissions().mode())
            },
        };
        serde_json::to_string_pretty(&info).map_err(|e| e.to_string())
    }

    /// List the allowed directories this server can access.
    #[tool(description = "List the directories that this server is allowed to access")]
    async fn list_allowed_directories(&self) -> String {
        self.allowed_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Build a simple unified diff between two strings.
fn build_diff(original: &str, modified: &str) -> String {
    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();
    let mut diff = String::new();

    let max_len = orig_lines.len().max(mod_lines.len());
    for i in 0..max_len {
        let orig = orig_lines.get(i);
        let modif = mod_lines.get(i);
        match (orig, modif) {
            (Some(o), Some(m)) if o != m => {
                diff.push_str(&format!("-{o}\n+{m}\n"));
            }
            (Some(o), Some(_)) => {
                diff.push_str(&format!(" {o}\n"));
            }
            (Some(o), None) => {
                diff.push_str(&format!("-{o}\n"));
            }
            (None, Some(m)) => {
                diff.push_str(&format!("+{m}\n"));
            }
            (None, None) => {}
        }
    }
    diff
}

/// Recursively build a tree of the filesystem.
fn build_tree(path: &Path) -> std::pin::Pin<Box<dyn Future<Output = Result<TreeNode, std::io::Error>> + Send + '_>> {
    Box::pin(async move {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let meta = tokio::fs::symlink_metadata(path).await?;
        if !meta.is_dir() {
            return Ok(TreeNode {
                name,
                node_type: "file",
                children: None,
            });
        }

        let mut children = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let child_path = entry.path();
            match build_tree(&child_path).await {
                Ok(child) => children.push(child),
                Err(_) => continue, // skip inaccessible entries
            }
        }
        children.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(TreeNode {
            name,
            node_type: "directory",
            children: Some(children),
        })
    })
}
