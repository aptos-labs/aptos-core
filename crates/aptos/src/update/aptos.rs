// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Out of the box the self_update crate assumes that you have releases named a
// specific way with the crate name, version, and target triple in a specific
// format. We don't do this with our releases, we have other GitHub releases beyond
// just the CLI, and we don't build for all major target triples, so we have to do
// some of the work ourselves first to figure out what the latest version of the
// CLI is and which binary to download based on the current OS. Then we can plug
// that into the library which takes care of the rest.

use super::{update_binary, BinaryUpdater, UpdateRequiredInfo};
use crate::common::{
    types::{CliCommand, CliTypedResult, PromptOptions, USER_AGENT},
    utils::cli_build_information,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use aptos_build_info::BUILD_OS;
use async_trait::async_trait;
use clap::Parser;
use reqwest::{
    blocking::{Client, RequestBuilder},
    header::{ACCEPT, LINK},
    StatusCode,
};
use self_update::{backends::github::Update, cargo_crate_version, update::ReleaseUpdate};
use semver::Version;
use serde::{de::IgnoredAny, Deserialize};
use std::{path::Path, time::Duration};

const REPO_OWNER: &str = "aptos-labs";
const REPO_NAME: &str = "aptos-core";
const CLI_TAG_PREFIX: &str = "aptos-cli-v";
/// Maximum number of stable tags to check, starting with the highest version.
const MAX_RELEASE_CANDIDATES: usize = 3;
const HOMEBREW_FORMULA_URL: &str = "https://formulae.brew.sh/api/formula/aptos.json";

/// Update the CLI itself
///
/// This can be used to update the CLI to the latest version. This is useful if you
/// installed the CLI via the install script / by downloading the binary directly.
#[derive(Debug, Parser)]
pub struct AptosUpdateTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = REPO_OWNER)]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = REPO_NAME)]
    repo_name: String,

    /// If set, it will check if there are updates for the tool, but not actually update
    #[clap(long, default_value_t = false)]
    check: bool,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl BinaryUpdater for AptosUpdateTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> String {
        "Aptos CLI".to_string()
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        let target_version = if self.repo_owner == REPO_OWNER && self.repo_name == REPO_NAME {
            InstallationMethod::from_env()?.fetch_latest()?
        } else {
            fetch_latest_github_release(&self.repo_owner, &self.repo_name)?
        };

        Ok(UpdateRequiredInfo {
            current_version: Some(cargo_crate_version!().to_string()),
            target_version: target_version.to_string(),
        })
    }

    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>> {
        let installation_method =
            InstallationMethod::from_env().context("Failed to determine installation method")?;
        match installation_method {
            InstallationMethod::Source => {
                return Err(anyhow!(
                    "Detected this CLI was built from source, refusing to update"
                ));
            },
            InstallationMethod::Homebrew => {
                return Err(anyhow!(
                    "Detected this CLI comes from homebrew, use `brew upgrade aptos` instead"
                ));
            },
            InstallationMethod::VersionManager => {
                return Err(anyhow!(
                    "Detected this CLI comes from a version manager (asdf or mise), use it to update instead"
                ));
            },
            InstallationMethod::PackageManager => {
                return Err(anyhow!(
                    "Detected this CLI comes from a package manager, use your package manager to update instead"
                ));
            },
            InstallationMethod::Other => {},
        }

        // Determine the target we should download. This is necessary because we don't
        // name our binary releases using the target triples nor do we build specifically
        // for all major triples, so we have to generalize to one of the binaries we do
        // happen to build. We figure this out based on what system the CLI was built on.
        let build_info = cli_build_information();
        let target = match build_info.get(BUILD_OS).context("Failed to determine build info of current CLI")?.as_str() {
            "linux-x86_64" => "Linux-x86_64",
            "linux-aarch64" => "Linux-aarch64",
            "macos-x86_64" => "macOS-x86_64",
            "macos-aarch64" => "macOS-arm64",
            "windows-x86_64" => "Windows-x86_64",
            wildcard => return Err(anyhow!("Self-updating is not supported on your OS ({}) right now, please download the binary manually", wildcard)),
        };

        let current_version = match &info.current_version {
            Some(version) => version,
            None => unreachable!("current_version should always be Some at this point"),
        };

        // Build a new configuration that will direct the library to download the
        // binary with the target version tag and target that we determined above.
        Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name("aptos")
            .current_version(current_version)
            .target_version_tag(&format!("{}{}", CLI_TAG_PREFIX, info.target_version))
            .target(target)
            .no_confirm(self.prompt_options.assume_yes)
            .build()
            .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallationMethod {
    Source,
    Homebrew,
    VersionManager,
    PackageManager,
    Other,
}

impl InstallationMethod {
    pub fn from_env() -> Result<Self> {
        // Release builds, including Homebrew's, are compiled without debug assertions.
        if cfg!(debug_assertions) {
            return Ok(InstallationMethod::Source);
        }
        let exe_path = std::env::current_exe()?;
        // Resolve symlinks so installation detection uses the binary's location.
        let exe_path = exe_path.canonicalize().unwrap_or(exe_path);
        Ok(Self::from_path(&exe_path))
    }

    fn from_path(exe_path: &Path) -> Self {
        let path = exe_path.to_string_lossy();
        // Cargo's output directory is always lowercase `target`; other locations vary in case.
        if path
            .split(['/', '\\'])
            .any(|component| component == "target")
        {
            return InstallationMethod::Source;
        }
        let path = path.to_ascii_lowercase();
        let components: Vec<&str> = path
            .split(['/', '\\'])
            .filter(|component| !component.is_empty())
            .collect();
        let has = |name: &str| components.contains(&name);

        if has("cellar") || has("homebrew") || has(".linuxbrew") || has("linuxbrew") {
            InstallationMethod::Homebrew
        } else if components
            .windows(2)
            .any(|pair| pair == ["installs", "aptos"])
            || (has("installs") && (has(".asdf") || has("mise")))
        {
            InstallationMethod::VersionManager
        } else if components.starts_with(&["usr", "bin"])
            || components.starts_with(&["nix", "store"])
            || (has("scoop") && has("apps"))
            || (has("chocolatey") && has("lib"))
            || (has("winget") && has("packages"))
        {
            InstallationMethod::PackageManager
        } else {
            InstallationMethod::Other
        }
    }

    /// Fetches the latest Homebrew formula version for Homebrew installs,
    /// or the latest GitHub release otherwise.
    pub(crate) fn fetch_latest(&self) -> Result<Version> {
        match self {
            InstallationMethod::Homebrew => fetch_latest_homebrew_release(),
            InstallationMethod::Source
            | InstallationMethod::VersionManager
            | InstallationMethod::PackageManager
            | InstallationMethod::Other => fetch_latest_github_release(REPO_OWNER, REPO_NAME),
        }
    }

    /// Returns upgrade instructions, or `None` to suppress notices for this installation.
    pub(crate) fn upgrade_hint(&self) -> Option<&'static str> {
        match self {
            InstallationMethod::Homebrew => Some("To upgrade, run: brew upgrade aptos"),
            InstallationMethod::VersionManager => Some(
                "To upgrade, use the version manager that installed the Aptos CLI (asdf or mise).",
            ),
            InstallationMethod::Other => Some("To upgrade, run: aptos update aptos"),
            InstallationMethod::Source | InstallationMethod::PackageManager => None,
        }
    }
}

#[derive(Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    name: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    prerelease: bool,
    assets: Vec<IgnoredAny>,
}

impl GitHubRelease {
    /// Stable tags may refer to releases marked as prereleases or still awaiting assets.
    /// Both are excluded from updates.
    fn is_installable(&self) -> bool {
        !self.prerelease && !self.assets.is_empty()
    }
}

#[derive(Deserialize)]
struct BrewFormula {
    versions: BrewVersions,
}

#[derive(Deserialize)]
struct BrewVersions {
    stable: String,
}

/// Parses `text` as a release version, rejecting pre-release and build suffixes.
pub(crate) fn parse_stable_version(text: &str) -> Option<Version> {
    Version::parse(text)
        .ok()
        .filter(|version| version.pre.is_empty() && version.build.is_empty())
}

/// Returns the stable CLI versions among `refs`, newest first.
fn stable_cli_versions(refs: &[GitRef]) -> Vec<Version> {
    let mut versions: Vec<Version> = refs
        .iter()
        .filter_map(|git_ref| git_ref.name.strip_prefix("refs/tags/"))
        .filter_map(|tag| tag.strip_prefix(CLI_TAG_PREFIX))
        .filter_map(parse_stable_version)
        .collect();
    versions.sort_unstable_by(|left, right| right.cmp(left));
    versions
}

fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()?)
}

fn github_get(client: &Client, url: String) -> RequestBuilder {
    client
        .get(url)
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
}

/// Finds an installable release among the newest `MAX_RELEASE_CANDIDATES` stable CLI tags.
/// Filtering tags by the CLI prefix avoids paging through other components' releases.
fn fetch_latest_github_release(owner: &str, repo: &str) -> Result<Version> {
    let client = http_client()?;
    let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let response = github_get(
        &client,
        format!("{}/git/matching-refs/tags/{}", repo_url, CLI_TAG_PREFIX),
    )
    .send()?
    .error_for_status()?;
    let has_next_page = response.headers().get_all(LINK).iter().any(|link| {
        link.to_str()
            .is_ok_and(|link| link.contains("rel=\"next\""))
    });
    ensure!(!has_next_page, "CLI release tags span multiple pages");
    let refs: Vec<GitRef> = response.json()?;

    for version in stable_cli_versions(&refs)
        .into_iter()
        .take(MAX_RELEASE_CANDIDATES)
    {
        let response = github_get(
            &client,
            format!("{}/releases/tags/{}{}", repo_url, CLI_TAG_PREFIX, version),
        )
        .send()?;
        // Unauthenticated requests cannot see draft releases.
        if response.status() == StatusCode::NOT_FOUND {
            continue;
        }
        let release: GitHubRelease = response.error_for_status()?.json()?;
        if release.is_installable() {
            return Ok(version);
        }
    }
    bail!("Failed to find a published CLI release")
}

fn fetch_latest_homebrew_release() -> Result<Version> {
    let formula: BrewFormula = http_client()?
        .get(HOMEBREW_FORMULA_URL)
        .send()?
        .error_for_status()?
        .json()?;
    parse_stable_version(&formula.versions.stable).ok_or_else(|| {
        anyhow!(
            "Unexpected Homebrew formula version: {}",
            formula.versions.stable
        )
    })
}

#[async_trait]
impl CliCommand<String> for AptosUpdateTool {
    fn command_name(&self) -> &'static str {
        "UpdateAptos"
    }

    async fn execute(self) -> CliTypedResult<String> {
        update_binary(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_cli_versions_newest_first() {
        let response = r#"[
            {
                "ref": "refs/tags/aptos-cli-v0.1.0-alpha",
                "node_id": "REF_kwDOG5KunrpyZWZzL3RhZ3MvYXB0b3MtY2xpLXYwLjEuMA",
                "url": "https://api.github.com/repos/aptos-labs/aptos-core/git/refs/tags/aptos-cli-v0.1.0-alpha",
                "object": {"sha": "3a253e7288fa3473cef2b8c2f7d7e32a9b3e8e52", "type": "tag", "url": "https://api.github.com/repos/aptos-labs/aptos-core/git/tags/3a253e7288fa3473cef2b8c2f7d7e32a9b3e8e52"}
            },
            {"ref": "refs/tags/aptos-cli-v0.3.1a"},
            {"ref": "refs/tags/aptos-cli-v10.0.0"},
            {"ref": "refs/tags/aptos-cli-v10.1.0-rc.1"},
            {"ref": "refs/tags/aptos-cli-v9.5.1"}
        ]"#;
        let refs: Vec<GitRef> = serde_json::from_str(response).expect("tag refs should parse");
        assert_eq!(stable_cli_versions(&refs), [
            Version::new(10, 0, 0),
            Version::new(9, 5, 1)
        ]);
    }

    #[test]
    fn github_release_installability() {
        let release = |prerelease: bool, assets: &str| {
            let response = format!(
                r#"{{
                    "url": "https://api.github.com/repos/aptos-labs/aptos-core/releases/394156499",
                    "id": 394156499,
                    "tag_name": "aptos-cli-v9.6.0",
                    "name": "Aptos CLI Release v9.6.0",
                    "draft": false,
                    "prerelease": {},
                    "assets": [{}],
                    "body": "Release notes"
                }}"#,
                prerelease, assets
            );
            serde_json::from_str::<GitHubRelease>(&response).expect("release should parse")
        };
        let asset = r#"{
            "id": 582354010,
            "name": "aptos-cli-9.6.0-Linux-aarch64.zip",
            "state": "uploaded",
            "size": 46955954,
            "browser_download_url": "https://github.com/aptos-labs/aptos-core/releases/download/aptos-cli-v9.6.0/aptos-cli-9.6.0-Linux-aarch64.zip"
        }"#;
        assert!(release(false, asset).is_installable());
        assert!(!release(false, "").is_installable());
        assert!(!release(true, asset).is_installable());
    }

    #[test]
    fn parse_stable_version_rejects_malformed_input() {
        for text in [
            "",
            "9",
            "9.5",
            "v9.5.1",
            "9.5.1a",
            "9.5.1-rc.1",
            "9.5.1+build",
            "latest",
        ] {
            assert_eq!(parse_stable_version(text), None, "{}", text);
        }
        assert_eq!(parse_stable_version("9.5.1"), Some(Version::new(9, 5, 1)));
    }

    #[test]
    fn homebrew_formula_stable_version() {
        let response = r#"{
            "name": "aptos",
            "full_name": "aptos",
            "tap": "homebrew/core",
            "versions": {"stable": "9.5.1", "head": "HEAD", "bottle": true},
            "revision": 0
        }"#;
        let formula: BrewFormula = serde_json::from_str(response).expect("formula should parse");
        assert_eq!(
            parse_stable_version(&formula.versions.stable),
            Some(Version::new(9, 5, 1))
        );
    }

    #[test]
    fn installation_method_from_path() {
        use InstallationMethod::*;
        let cases = [
            ("/usr/local/Cellar/aptos/9.5.1/bin/aptos", Homebrew),
            ("/opt/homebrew/Cellar/aptos/9.5.1/bin/aptos", Homebrew),
            (
                "/home/linuxbrew/.linuxbrew/Cellar/aptos/9.5.1/bin/aptos",
                Homebrew,
            ),
            ("/Users/brewer/.local/bin/aptos", Other),
            ("/home/targetuser/.local/bin/aptos", Other),
            ("/Users/Target/.local/bin/aptos", Other),
            ("/repo/target/release/aptos", Source),
            (
                "/home/u/.asdf/installs/aptos/9.5.1/bin/aptos",
                VersionManager,
            ),
            (
                "/home/u/.local/share/mise/installs/aptos/9.5.1/bin/aptos",
                VersionManager,
            ),
            (
                "/home/u/.local/share/asdf/installs/aptos/9.5.1/bin/aptos",
                VersionManager,
            ),
            (
                "/nix/store/0c8vh8wbdzw9k4d7x5lbhkmjdnqrn2vn-aptos-9.5.1/bin/aptos",
                PackageManager,
            ),
            ("/usr/bin/aptos", PackageManager),
            (
                r"C:\Users\u\scoop\apps\aptos\current\aptos.exe",
                PackageManager,
            ),
            (
                r"C:\ProgramData\chocolatey\lib\aptos\tools\aptos.exe",
                PackageManager,
            ),
            (
                r"C:\Users\u\AppData\Local\Microsoft\WinGet\Packages\Aptos.Aptos_x\aptos.exe",
                PackageManager,
            ),
            (r"C:\Users\u\.aptoscli\bin\aptos.exe", Other),
        ];
        for (path, expected) in cases {
            assert_eq!(
                InstallationMethod::from_path(Path::new(path)),
                expected,
                "{}",
                path
            );
        }
    }
}
