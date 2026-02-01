use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Secret file patterns that should always be excluded
const SECRET_PATTERNS: &[&str] = &[
    ".env",
    "credentials.json",
    "serviceaccount.json",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
];

/// Secret substring patterns (case-insensitive)
const SECRET_SUBSTRINGS: &[&str] = &["secret", "password", "credential"];

/// File extensions that indicate binary/non-text files
const BINARY_EXTENSIONS: &[&str] = &[
    // Images
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp",
    // Media
    "mp4", "mp3", "wav", "avi", "mov", "flac", "ogg",
    // Archives
    "zip", "tar", "gz", "7z", "rar", "bz2", "xz",
    // Binaries
    "exe", "dll", "so", "dylib", "bin",
    // Compiled
    "wasm", "class", "pyc", "o", "a", "lib",
    // Other
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
];

#[derive(Debug, Clone, PartialEq)]
pub enum SkipReason {
    Secret,
    Binary,
    TooLarge,
    Extension,
    Gitignore,
    ReadError,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::Secret => write!(f, "secret"),
            SkipReason::Binary => write!(f, "binary"),
            SkipReason::TooLarge => write!(f, "too large"),
            SkipReason::Extension => write!(f, "extension"),
            SkipReason::Gitignore => write!(f, "gitignore"),
            SkipReason::ReadError => write!(f, "read error"),
        }
    }
}

/// Check if a filename matches secret patterns
pub fn is_secret_file(path: &Path) -> bool {
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy().to_lowercase(),
        None => return false,
    };

    // Check exact patterns
    if SECRET_PATTERNS.iter().any(|p| file_name == *p) {
        return true;
    }

    // Check .env variants
    if file_name.starts_with(".env") {
        return true;
    }

    // Check extensions
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if matches!(ext.as_str(), "key" | "pem" | "p12" | "pfx") {
            return true;
        }
    }

    // Check substrings
    SECRET_SUBSTRINGS
        .iter()
        .any(|s| file_name.contains(s))
}

/// Check if a file extension indicates a binary file
pub fn is_binary_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        return BINARY_EXTENSIONS.contains(&ext.as_str());
    }
    false
}

/// Check if a file is binary by reading its content
/// Returns true if the file appears to be binary (contains null bytes in first 8KB)
pub fn is_binary_content(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut buffer = vec![0; 8192];
    match file.read(&mut buffer) {
        Ok(n) => {
            // Check for null bytes in the read portion
            buffer[..n].contains(&0)
        }
        Err(_) => false,
    }
}

/// Check if a file exceeds the size limit
pub fn exceeds_size_limit(path: &Path, max_size: u64) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() > max_size,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_file_detection() {
        assert!(is_secret_file(Path::new(".env")));
        assert!(is_secret_file(Path::new(".env.local")));
        assert!(is_secret_file(Path::new(".env.production")));
        assert!(is_secret_file(Path::new("credentials.json")));
        assert!(is_secret_file(Path::new("id_rsa")));
        assert!(is_secret_file(Path::new("my.key")));
        assert!(is_secret_file(Path::new("cert.pem")));
        assert!(is_secret_file(Path::new("my-secret-file.txt")));
        assert!(is_secret_file(Path::new("passwords.txt")));

        assert!(!is_secret_file(Path::new("main.rs")));
        assert!(!is_secret_file(Path::new("config.toml")));
    }

    #[test]
    fn test_binary_extension_detection() {
        assert!(is_binary_extension(Path::new("image.png")));
        assert!(is_binary_extension(Path::new("logo.jpg")));
        assert!(is_binary_extension(Path::new("output.wasm")));
        assert!(is_binary_extension(Path::new("archive.zip")));
        assert!(is_binary_extension(Path::new("binary.exe")));

        assert!(!is_binary_extension(Path::new("main.rs")));
        assert!(!is_binary_extension(Path::new("config.toml")));
        assert!(!is_binary_extension(Path::new("README.md")));
    }
}
