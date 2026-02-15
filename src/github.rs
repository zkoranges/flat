use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use tempfile::TempDir;
use url::Url;

/// GitHub repository specification
#[derive(Debug, Clone)]
pub struct GitHubUrl {
    pub owner: String,
    pub repo: String,
    pub branch: Option<String>,
    pub subpath: Option<String>,
}

/// Result of cloning a repository
pub enum CloneResult {
    Success { path: PathBuf, temp_dir: TempDir },
    GitBinaryFallback { path: PathBuf, temp_dir: TempDir },
}

impl CloneResult {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Success { path, .. } | Self::GitBinaryFallback { path, .. } => path,
        }
    }

    pub fn into_path_and_temp(self) -> (PathBuf, TempDir) {
        match self {
            Self::Success { path, temp_dir } | Self::GitBinaryFallback { path, temp_dir } => {
                (path, temp_dir)
            }
        }
    }
}

/// Parse GitHub URL in various formats
pub fn parse_github_url(input: &str) -> Result<GitHubUrl> {
    let input = input.trim();

    // Format 1: owner/repo
    if !input.contains("://") && !input.starts_with("git@") && input.contains('/') {
        let parts: Vec<&str> = input.split('/').collect();
        if parts.len() >= 2 {
            return Ok(GitHubUrl {
                owner: parts[0].to_string(),
                repo: parts[1]
                    .trim_end_matches(".git")
                    .split('/')
                    .next()
                    .unwrap_or(parts[1])
                    .to_string(),
                branch: if parts.len() > 2 && parts[2] == "tree" && parts.len() > 3 {
                    Some(parts[3].to_string())
                } else {
                    None
                },
                subpath: if parts.len() > 4 && parts[2] == "tree" {
                    Some(parts[4..].join("/"))
                } else {
                    None
                },
            });
        }
    }

    // Format 2: git@github.com:owner/repo.git
    if input.starts_with("git@github.com:") {
        let rest = input.strip_prefix("git@github.com:").unwrap();
        let parts: Vec<&str> = rest.trim_end_matches(".git").split('/').collect();
        if parts.len() >= 2 {
            return Ok(GitHubUrl {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
                branch: None,
                subpath: None,
            });
        }
    }

    // Format 3: https://github.com/owner/repo or variations
    if let Ok(url) = Url::parse(input) {
        if url.host_str().is_some_and(|h| h.contains("github.com")) {
            let path = url.path();
            let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

            if parts.len() >= 2 {
                let owner = parts[0].to_string();
                let repo_with_git = parts[1];
                let repo = repo_with_git
                    .trim_end_matches(".git")
                    .to_string();

                let (branch, subpath) = if parts.len() > 2 && parts[2] == "tree" && parts.len() > 3 {
                    let br = parts[3].to_string();
                    let sp = if parts.len() > 4 {
                        Some(parts[4..].join("/"))
                    } else {
                        None
                    };
                    (Some(br), sp)
                } else {
                    (None, None)
                };

                return Ok(GitHubUrl {
                    owner,
                    repo,
                    branch,
                    subpath,
                });
            }
        }
    }

    Err(anyhow!(
        "Invalid GitHub URL format: '{}'. \
         Supported formats: owner/repo, https://github.com/owner/repo, git@github.com:owner/repo.git",
        input
    ))
}

/// Clone a GitHub repository
pub fn clone_repo(url: &GitHubUrl, token: Option<&str>) -> Result<CloneResult> {
    let temp_dir = TempDir::new().context("Failed to create temporary directory")?;
    let temp_path = temp_dir.path();

    // Build the repository URL
    let repo_url = if let Some(token) = token {
        format!(
            "https://{}@github.com/{}/{}.git",
            token, url.owner, url.repo
        )
    } else {
        format!("https://github.com/{}/{}.git", url.owner, url.repo)
    };

    // Try git2 first with shallow clone
    if let Ok(result) = try_clone_with_git2(&repo_url, temp_path, &url.branch) {
        return Ok(CloneResult::Success {
            path: result,
            temp_dir,
        });
    }

    // Fallback to git binary
    eprintln!("Note: git2 clone failed, falling back to git binary");
    let result = try_clone_with_git_binary(&repo_url, temp_path, &url.branch)
        .context("Failed to clone repository using git binary")?;

    Ok(CloneResult::GitBinaryFallback {
        path: result,
        temp_dir,
    })
}

/// Try to clone using git2 library
fn try_clone_with_git2(
    repo_url: &str,
    temp_path: &std::path::Path,
    _branch: &Option<String>,
) -> Result<PathBuf> {
    // For shallow clones, we need to use fetch instead of clone directly
    // For now, just do a basic clone
    git2::Repository::clone(repo_url, temp_path)
        .context("git2 clone failed")?;

    Ok(temp_path.to_path_buf())
}

/// Try to clone using git binary
fn try_clone_with_git_binary(
    repo_url: &str,
    temp_path: &std::path::Path,
    branch: &Option<String>,
) -> Result<PathBuf> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone");

    // Add shallow clone flag for faster cloning
    cmd.arg("--depth").arg("1");

    // Add branch if specified
    if let Some(br) = branch {
        cmd.arg("--branch").arg(br);
    }

    cmd.arg(repo_url).arg(temp_path);

    let output = cmd
        .output()
        .context("Failed to execute git clone command")?;

    if !output.status.success() {
        return Err(anyhow!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(temp_path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_short_format() {
        let url = parse_github_url("zkoranges/flat").unwrap();
        assert_eq!(url.owner, "zkoranges");
        assert_eq!(url.repo, "flat");
        assert_eq!(url.branch, None);
    }

    #[test]
    fn test_parse_https_url() {
        let url = parse_github_url("https://github.com/zkoranges/flat").unwrap();
        assert_eq!(url.owner, "zkoranges");
        assert_eq!(url.repo, "flat");
        assert_eq!(url.branch, None);
    }

    #[test]
    fn test_parse_git_ssh_url() {
        let url = parse_github_url("git@github.com:zkoranges/flat.git").unwrap();
        assert_eq!(url.owner, "zkoranges");
        assert_eq!(url.repo, "flat");
    }

    #[test]
    fn test_parse_with_branch() {
        let url = parse_github_url("https://github.com/zkoranges/flat/tree/main").unwrap();
        assert_eq!(url.owner, "zkoranges");
        assert_eq!(url.repo, "flat");
        assert_eq!(url.branch, Some("main".to_string()));
    }

    #[test]
    fn test_parse_with_branch_and_subpath() {
        let url =
            parse_github_url("https://github.com/zkoranges/flat/tree/main/src").unwrap();
        assert_eq!(url.owner, "zkoranges");
        assert_eq!(url.repo, "flat");
        assert_eq!(url.branch, Some("main".to_string()));
        assert_eq!(url.subpath, Some("src".to_string()));
    }

    #[test]
    fn test_invalid_url() {
        let result = parse_github_url("not a valid url");
        assert!(result.is_err());
    }
}
