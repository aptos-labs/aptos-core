use std::path::PathBuf;
use anyhow::anyhow;

pub const REVELA_BINARY_NAME: &str = "revela";
pub const TARGET_REVELA_VERSION: &str = "1.0.0";

const REVELA_EXE_ENV: &str = "REVELA_EXE";
#[cfg(target_os = "windows")]
const REVELA_EXE: &str = "revela.exe";
#[cfg(not(target_os = "windows"))]
const REVELA_EXE: &str = "revela";

pub fn get_revela_path() -> anyhow::Result<PathBuf> {
    get_path(
        "decompiler",
        REVELA_EXE_ENV,
        REVELA_BINARY_NAME,
        REVELA_EXE,
        false,
    )
}

pub const FORMATTER_BINARY_NAME: &str = "movefmt";
pub const TARGET_FORMATTER_VERSION: &str = "1.3.7";

const FORMATTER_EXE_ENV: &str = "FORMATTER_EXE";
#[cfg(target_os = "windows")]
const FORMATTER_EXE: &str = "movefmt.exe";
#[cfg(not(target_os = "windows"))]
const FORMATTER_EXE: &str = "movefmt";

pub fn get_movefmt_path() -> anyhow::Result<PathBuf> {
    get_path(
        FORMATTER_BINARY_NAME,
        FORMATTER_EXE_ENV,
        FORMATTER_BINARY_NAME,
        FORMATTER_EXE,
        true,
    )
}

pub fn get_path(
    name: &str,
    exe_env: &str,
    binary_name: &str,
    exe: &str,
    find_in_path: bool,
) -> anyhow::Result<PathBuf> {
    // Look at the environment variable first.
    if let Ok(path) = std::env::var(exe_env) {
        return Ok(PathBuf::from(path));
    }

    // See if it is present in the path where we usually install additional binaries.
    let path = get_additional_binaries_dir().join(binary_name);
    if path.exists() && path.is_file() {
        return Ok(path);
    }

    if find_in_path {
        // See if we can find the binary in the PATH.
        if let Some(path) = pathsearch::find_executable_in_path(exe) {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "Cannot locate the {} executable. \
            Environment variable `{}` is not set, and `{}` is not in the PATH. \
            Try running `aptos update {}` to download it and then \
            updating the environment variable `{}` or adding the executable to PATH",
        name,
        exe_env,
        exe,
        exe,
        exe_env
    ))
}

/// Some functionality of the Aptos CLI relies on some additional binaries. This is
/// where we install them by default. These paths align with the installation script,
/// which is generally how the Linux and Windows users install the CLI.
pub fn get_additional_binaries_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "".into());
        PathBuf::from(home_dir).join(".aptoscli/bin")
    }

    #[cfg(not(windows))]
    {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "".into());
        PathBuf::from(home_dir).join(".local/bin")
    }
}
