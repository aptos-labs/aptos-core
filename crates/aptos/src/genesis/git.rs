// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliError, CliTypedResult},
        utils::write_to_file,
    },
    genesis::config::Layout,
    CliCommand,
};
use aptos_config::config::Token;
use aptos_github_client::Client as GithubClient;
use async_trait::async_trait;
use clap::Parser;
use serde::{de::DeserializeOwned, Serialize};
use std::{io::Read, path::PathBuf, str::FromStr};

pub const LAYOUT_NAME: &str = "layout";

/// Setup a shared Github repository for Genesis
///
#[derive(Parser)]
pub struct SetupGit {
    #[clap(flatten)]
    git_options: GitOptions,
    /// Path to `Layout` which defines where all the files are
    #[clap(long, parse(from_os_str))]
    layout_file: PathBuf,
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
        client.put(LAYOUT_NAME, &layout)?;

        // Make a place for the modules to be uploaded
        client.create_dir("framework")?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
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
                owner: parts.get(0).unwrap().to_string(),
                repository: parts.get(1).unwrap().to_string(),
            })
        }
    }
}

#[derive(Clone, Parser)]
pub struct GitOptions {
    /// Github repository e.g. 'aptos-labs/aptos-core'
    #[clap(long)]
    github_repository: Option<GithubRepo>,
    /// Github repository branch e.g. main
    #[clap(long, default_value = "main")]
    github_branch: String,
    /// Path to Github API token.  Token must have repo:* permissions
    #[clap(long, parse(from_os_str))]
    github_token_file: Option<PathBuf>,
    /// Path to local git repository
    #[clap(long, parse(from_os_str))]
    local_repository_dir: Option<PathBuf>,
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
    pub fn get<T: DeserializeOwned>(&self, name: &str) -> CliTypedResult<T> {
        match self {
            Client::Local(local_repository_path) => {
                let path = local_repository_path.join(format!("{}.yaml", name));
                let mut file = std::fs::File::open(path.as_path())
                    .map_err(|e| CliError::IO(path.display().to_string(), e))?;

                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| CliError::IO(path.display().to_string(), e))?;
                from_yaml(&contents)
            }
            Client::Github(client) => {
                from_base64_encoded_yaml(&client.get_file(&format!("{}.yaml", name))?)
            }
        }
    }

    /// Puts an object as a YAML encoded file to the appropriate storage
    pub fn put<T: Serialize + ?Sized>(&self, name: &str, input: &T) -> CliTypedResult<()> {
        match self {
            Client::Local(local_repository_path) => {
                self.create_dir(local_repository_path.to_str().unwrap())?;

                let path = local_repository_path.join(format!("{}.yaml", name));
                write_to_file(
                    path.as_path(),
                    &path.display().to_string(),
                    to_yaml(input)?.as_bytes(),
                )?;
            }
            Client::Github(client) => {
                client.put(&format!("{}.yaml", name), &to_base64_encoded_yaml(input)?)?;
            }
        }

        Ok(())
    }

    pub fn create_dir(&self, name: &str) -> CliTypedResult<()> {
        match self {
            Client::Local(local_repository_path) => {
                let path = local_repository_path.join(name);
                if !path.exists() || !path.is_dir() {
                    std::fs::create_dir(path.as_path())
                        .map_err(|e| CliError::IO(path.display().to_string(), e))?
                };
            }
            Client::Github(_) => {
                // There's no such thing as an empty directory in Git, so do nothing
            }
        }

        Ok(())
    }

    /// Retrieve bytecode Move modules from a module folder
    pub fn get_modules(&self, name: &str) -> CliTypedResult<Vec<Vec<u8>>> {
        let mut modules = Vec::new();

        match self {
            Client::Local(local_repository_path) => {
                let module_folder = local_repository_path.join(name);
                if !module_folder.is_dir() {
                    return Err(CliError::UnexpectedError(format!(
                        "{} is not a directory!",
                        module_folder.display()
                    )));
                }

                let files = std::fs::read_dir(module_folder.as_path())
                    .map_err(|e| CliError::IO(module_folder.display().to_string(), e))?;

                for maybe_file in files {
                    let file = maybe_file
                        .map_err(|e| CliError::UnexpectedError(e.to_string()))?
                        .path();
                    let extension = file.extension();

                    // Only collect move files
                    if file.is_file() && extension.is_some() && extension.unwrap() == "mv" {
                        modules.push(
                            std::fs::read(file.as_path())
                                .map_err(|e| CliError::IO(file.display().to_string(), e))?,
                        );
                    }
                }
            }
            Client::Github(client) => {
                let files = client.get_directory(name)?;

                for file in files {
                    // Only collect .mv files
                    if file.ends_with(".mv") {
                        modules.push(base64::decode(client.get_file(&file)?)?)
                    }
                }
            }
        }
        Ok(modules)
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
