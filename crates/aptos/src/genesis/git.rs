// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::utils::create_dir_if_not_exist;
use crate::{
    common::{
        types::{CliError, CliTypedResult},
        utils::write_to_file,
    },
    CliCommand,
};
use aptos_config::config::Token;
use aptos_genesis::config::Layout;
use aptos_github_client::Client as GithubClient;
use async_trait::async_trait;
use clap::Parser;
use framework::ReleaseBundle;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::{fmt::Debug, io::Read, path::PathBuf, str::FromStr};

pub const LAYOUT_FILE: &str = "layout.yaml";
pub const OPERATOR_FILE: &str = "operator.yaml";
pub const OWNER_FILE: &str = "owner.yaml";
pub const FRAMEWORK_NAME: &str = "framework.mrb";
pub const BALANCES_FILE: &str = "balances.yaml";
pub const EMPLOYEE_VESTING_ACCOUNTS_FILE: &str = "employee_vesting_accounts.yaml";

/// Setup a shared Git repository for Genesis
///
/// This will setup a folder or an online Github repository to be used
/// for Genesis.  If it's the local, it will create the folders but not
/// set up a Git repository.
#[derive(Parser)]
pub struct SetupGit {
    #[clap(flatten)]
    pub(crate) git_options: GitOptions,

    /// Path to the `Layout` file which defines where all the files are
    #[clap(long, parse(from_os_str))]
    pub(crate) layout_file: PathBuf,
}

#[async_trait]
impl CliCommand<()> for SetupGit {
    fn command_name(&self) -> &'static str {
        "SetupGit"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let layout = Layout::from_disk(&self.layout_file)?;

        // Upload layout file to ensure we can read later
        let client = self.git_options.get_client()?;
        client.put(Path::new(LAYOUT_FILE), &layout)?;

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct GithubRepo {
    owner: String,
    repository: String,
}

impl FromStr for GithubRepo {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('/').collect();
        if parts.len() != 2 {
            Err(CliError::CommandArgumentError("Invalid repository must be of the form 'owner/repository` e.g. 'aptos-labs/aptos-core'".to_string()))
        } else {
            Ok(GithubRepo {
                owner: parts.first().unwrap().to_string(),
                repository: parts.get(1).unwrap().to_string(),
            })
        }
    }
}

#[derive(Clone, Default, Parser)]
pub struct GitOptions {
    /// Github repository e.g. 'aptos-labs/aptos-core'
    #[clap(long)]
    pub(crate) github_repository: Option<GithubRepo>,

    /// Github repository branch e.g. main
    #[clap(long, default_value = "main")]
    pub(crate) github_branch: String,

    /// Path to Github API token.  Token must have repo:* permissions
    #[clap(long, parse(from_os_str))]
    pub(crate) github_token_file: Option<PathBuf>,

    /// Path to local git repository
    #[clap(long, parse(from_os_str))]
    pub(crate) local_repository_dir: Option<PathBuf>,
}

impl GitOptions {
    pub fn get_client(self) -> CliTypedResult<Client> {
        if self.github_repository.is_none()
            && self.github_token_file.is_none()
            && self.local_repository_dir.is_some()
        {
            Ok(Client::local(self.local_repository_dir.unwrap()))
        } else if self.github_repository.is_some()
            && self.github_token_file.is_some()
            && self.local_repository_dir.is_none()
        {
            Client::github(
                self.github_repository.unwrap(),
                self.github_branch,
                self.github_token_file.unwrap(),
            )
        } else {
            Err(CliError::CommandArgumentError("Must provide either only --local-repository-dir or both --github-repository and --github-token-path".to_string()))
        }
    }
}

/// A client for abstracting away local vs Github storage
///
/// Note: Writes do not commit locally
pub enum Client {
    Local(PathBuf),
    Github(GithubClient),
}

impl Client {
    pub fn local(path: PathBuf) -> Client {
        Client::Local(path)
    }

    pub fn github(
        repository: GithubRepo,
        branch: String,
        token_path: PathBuf,
    ) -> CliTypedResult<Client> {
        let token = Token::FromDisk(token_path).read_token()?;
        Ok(Client::Github(GithubClient::new(
            repository.owner,
            repository.repository,
            branch,
            token,
        )))
    }

    /// Retrieves an object as a YAML encoded file from the appropriate storage
    pub fn get<T: DeserializeOwned + Debug>(&self, path: &Path) -> CliTypedResult<T> {
        match self {
            Client::Local(local_repository_path) => {
                let path = local_repository_path.join(path);
                let mut file = std::fs::File::open(path.as_path())
                    .map_err(|e| CliError::IO(path.display().to_string(), e))?;

                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| CliError::IO(path.display().to_string(), e))?;
                from_yaml(&contents)
            }
            Client::Github(client) => {
                from_base64_encoded_yaml(&client.get_file(&path.display().to_string())?)
            }
        }
    }

    /// Puts an object as a YAML encoded file to the appropriate storage
    pub fn put<T: Serialize + ?Sized>(&self, name: &Path, input: &T) -> CliTypedResult<()> {
        match self {
            Client::Local(local_repository_path) => {
                let path = local_repository_path.join(name);

                // Create repository path and any sub-directories
                if let Some(dir) = path.parent() {
                    self.create_dir(dir)?;
                } else {
                    return Err(CliError::UnexpectedError(format!(
                        "Path should always have a parent {}",
                        path.display()
                    )));
                }
                write_to_file(
                    path.as_path(),
                    &path.display().to_string(),
                    to_yaml(input)?.as_bytes(),
                )?;
            }
            Client::Github(client) => {
                client.put(&name.display().to_string(), &to_base64_encoded_yaml(input)?)?;
            }
        }

        Ok(())
    }

    pub fn create_dir(&self, dir: &Path) -> CliTypedResult<()> {
        match self {
            Client::Local(local_repository_path) => {
                let path = local_repository_path.join(dir);
                create_dir_if_not_exist(path.as_path())?;
            }
            Client::Github(_) => {
                // There's no such thing as an empty directory in Git, so do nothing
            }
        }

        Ok(())
    }

    /// Retrieve framework release bundle.
    pub fn get_framework(&self) -> CliTypedResult<ReleaseBundle> {
        match self {
            Client::Local(local_repository_path) => Ok(ReleaseBundle::read(
                local_repository_path.join(FRAMEWORK_NAME),
            )?),
            Client::Github(client) => {
                let bytes = base64::decode(client.get_file(FRAMEWORK_NAME)?)?;
                Ok(bcs::from_bytes::<ReleaseBundle>(&bytes)?)
            }
        }
    }
}

pub fn to_yaml<T: Serialize + ?Sized>(input: &T) -> CliTypedResult<String> {
    Ok(serde_yaml::to_string(input)?)
}

pub fn from_yaml<T: DeserializeOwned>(input: &str) -> CliTypedResult<T> {
    Ok(serde_yaml::from_str(input)?)
}

pub fn to_base64_encoded_yaml<T: Serialize + ?Sized>(input: &T) -> CliTypedResult<String> {
    Ok(base64::encode(to_yaml(input)?))
}

pub fn from_base64_encoded_yaml<T: DeserializeOwned>(input: &str) -> CliTypedResult<T> {
    from_yaml(&String::from_utf8(base64::decode(input)?)?)
}
