// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{velor_framework_path, components::ProposalMetadata, ExecutionMode, ReleaseConfig};
use anyhow::Result;
use velor::{
    common::types::CliCommand,
    governance::{ExecuteProposal, SubmitProposal, SubmitVote},
    move_tool::{RunFunction, RunScript},
    stake::IncreaseLockup,
};
use velor_crypto::ed25519::Ed25519PrivateKey;
use velor_genesis::keys::PrivateIdentity;
use velor_temppath::TempPath;
use velor_types::account_address::AccountAddress;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use url::Url;

pub const FAST_RESOLUTION_TIME: u64 = 30;
pub const DEFAULT_RESOLUTION_TIME: u64 = 43200;

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub endpoint: Url,
    pub root_key_path: PathBuf,
    pub validator_account: AccountAddress,
    pub validator_key: Ed25519PrivateKey,
    pub framework_git_rev: Option<String>,
}

impl NetworkConfig {
    pub fn new_from_dir(endpoint: Url, test_dir: &Path) -> Result<Self> {
        let root_key_path = test_dir.join("mint.key");
        let private_identity_file = test_dir.join("0/private-identity.yaml");
        let private_identity =
            serde_yaml::from_slice::<PrivateIdentity>(&fs::read(private_identity_file)?)?;

        Ok(Self {
            endpoint,
            root_key_path,
            validator_account: private_identity.account_address,
            validator_key: private_identity.account_private_key,
            framework_git_rev: None,
        })
    }

    // ED25519 Private keys have a very silly to_string that returns what looks
    // like a debug output:
    //   <elided secret for Ed25519PrivateKey>
    // Which is almost never what you want unless you're debugging...
    // Usually you want the hex encoded version, although you wanna be careful
    // you're not accidentally leaking private keys, but thats a separate issue
    pub fn get_hex_encoded_validator_key(&self) -> String {
        hex::encode(self.validator_key.to_bytes())
    }

    /// Submit all govenerance proposal script inside script_path to the corresponding rest endpoint.
    ///
    /// For all script, we will:
    /// - Generate a governance proposal and get its proposal id
    /// - Use validator's privkey to vote for this proposal
    /// - Add the proposal to allow list using validator account
    /// - Execute this proposal
    ///
    /// We expect all the scripts here to be single step governance proposal.
    pub async fn submit_and_execute_proposal(
        &self,
        metadata: &ProposalMetadata,
        script_path: Vec<PathBuf>,
        node_api_key: Option<String>,
    ) -> Result<()> {
        let mut proposals = vec![];
        for path in script_path.iter() {
            let proposal_id = self
                .create_governance_proposal(path.as_path(), metadata, false, node_api_key.clone())
                .await?;
            self.vote_proposal(proposal_id, node_api_key.clone())
                .await?;
            proposals.push(proposal_id);
        }

        // Wait for the voting period to pass
        sleep(Duration::from_secs(40));
        for (proposal_id, path) in proposals.iter().zip(script_path.iter()) {
            self.add_proposal_to_allow_list(*proposal_id, node_api_key.clone())
                .await?;
            self.execute_proposal(*proposal_id, path.as_path(), node_api_key.clone())
                .await?;
        }
        Ok(())
    }

    /// Submit all govenerance proposal script inside script_path to the corresponding rest endpoint.
    ///
    /// - We will first submit a governance proposal for the first script (in alphabetical order).
    /// - Validator will vote for this proposal
    ///
    /// Once voting period has passed, we should be able to execute all the scripts in the folder in alphabetical order.
    /// We expect all the scripts here to be multi step governance proposal.
    pub async fn submit_and_execute_multi_step_proposal(
        &self,
        metadata: &ProposalMetadata,
        script_path: Vec<PathBuf>,
        node_api_key: Option<String>,
    ) -> Result<()> {
        let first_script = script_path.first().unwrap();
        let proposal_id = self
            .create_governance_proposal(
                first_script.as_path(),
                metadata,
                true,
                node_api_key.clone(),
            )
            .await?;
        self.vote_proposal(proposal_id, node_api_key.clone())
            .await?;
        // Wait for the proposal to resolve.
        sleep(Duration::from_secs(40));
        for path in script_path {
            self.add_proposal_to_allow_list(proposal_id, node_api_key.clone())
                .await?;
            self.execute_proposal(proposal_id, path.as_path(), node_api_key.clone())
                .await?;
        }
        Ok(())
    }

    /// Change the time for a network to resolve governance proposal
    pub async fn set_fast_resolve(&self, resolution_time: u64) -> Result<()> {
        let fast_resolve_script = velor_temppath::TempPath::new();
        fast_resolve_script.create_as_file()?;
        let mut fas_script_path = fast_resolve_script.path().to_path_buf();
        fas_script_path.set_extension("move");

        std::fs::write(fas_script_path.as_path(), format!(r#"
        script {{
            use velor_framework::velor_governance;

            fun main(core_resources: &signer) {{
                let core_signer = velor_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);

                let framework_signer = &core_signer;

                velor_governance::update_governance_config(framework_signer, 0, 0, {});
            }}
        }}
        "#, resolution_time).as_bytes())?;

        let mut args = vec![
            "",
            "--script-path",
            fas_script_path.as_path().to_str().unwrap(),
            "--sender-account",
            "0xa550c18",
            "--private-key-file",
            self.root_key_path.as_path().to_str().unwrap(),
            "--assume-yes",
            "--encoding",
            "bcs",
            "--url",
            self.endpoint.as_str(),
        ];
        let rev = self.framework_git_rev.clone();
        let framework_path = velor_framework_path();
        if let Some(rev) = &rev {
            args.push("--framework-git-rev");
            args.push(rev.as_str());
        } else {
            args.push("--framework-local-dir");
            args.push(framework_path.as_os_str().to_str().unwrap());
        };

        RunScript::try_parse_from(args)?.execute().await?;
        Ok(())
    }

    pub async fn create_governance_proposal(
        &self,
        script_path: &Path,
        metadata: &ProposalMetadata,
        is_multi_step: bool,
        node_api_key: Option<String>,
    ) -> Result<u64> {
        println!("Creating proposal: {:?}", script_path);

        let address_string = format!("{}", self.validator_account);
        let privkey_string = self.get_hex_encoded_validator_key();

        let metadata_path = TempPath::new();
        metadata_path.create_as_file()?;
        fs::write(
            metadata_path.path(),
            serde_json::to_string_pretty(metadata)?,
        )?;

        let mut args = vec![
            "",
            "--pool-address",
            address_string.as_str(),
            "--script-path",
            script_path.to_str().unwrap(),
            "--metadata-path",
            metadata_path.path().to_str().unwrap(),
            "--metadata-url",
            "https://raw.githubusercontent.com/velor-chain/velor-core/b4fb9acfc297327c43d030def2b59037c4376611/testsuite/smoke-test/src/upgrade_multi_step_test_metadata.txt",
            "--sender-account",
            address_string.as_str(),
            "--private-key",
            privkey_string.as_str(),
            "--url",
            self.endpoint.as_str(),
            "--assume-yes",
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        if is_multi_step {
            args.push("--is-multi-step");
        }

        let rev_string = self.framework_git_rev.clone();
        let framework_path = velor_framework_path();
        let proposal_summary = if let Some(rev) = &rev_string {
            args.push("--framework-git-rev");
            args.push(rev.as_str());
            SubmitProposal::try_parse_from(args)?.execute().await?
        } else {
            args.push("--framework-local-dir");
            args.push(framework_path.as_os_str().to_str().unwrap());
            SubmitProposal::try_parse_from(args)?.execute().await?
        };

        Ok(proposal_summary
            .proposal_id
            .expect("Failed to extract proposal id"))
    }

    pub async fn vote_proposal(
        &self,
        proposal_id: u64,
        node_api_key: Option<String>,
    ) -> Result<()> {
        println!("Voting proposal id {:?}", proposal_id);

        let address_string = format!("{}", self.validator_account);
        let privkey_string = self.get_hex_encoded_validator_key();
        let proposal_id = format!("{}", proposal_id);

        let mut args = vec![
            "",
            "--pool-addresses",
            address_string.as_str(),
            "--sender-account",
            address_string.as_str(),
            "--private-key",
            privkey_string.as_str(),
            "--assume-yes",
            "--proposal-id",
            proposal_id.as_str(),
            "--yes",
            "--url",
            self.endpoint.as_str(),
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        SubmitVote::try_parse_from(args)?.execute().await?;
        Ok(())
    }

    pub async fn mint_to_validator(&self, node_api_key: Option<String>) -> Result<()> {
        let address_args = format!("address:{}", self.validator_account);

        println!("Minting to validator account");
        let mut args = vec![
            "",
            "--function-id",
            "0x1::velor_coin::mint",
            "--sender-account",
            "0xa550c18",
            "--args",
            address_args.as_str(),
            "u64:100000000000",
            "--private-key-file",
            self.root_key_path.as_path().to_str().unwrap(),
            "--assume-yes",
            "--encoding",
            "bcs",
            "--url",
            self.endpoint.as_str(),
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        RunFunction::try_parse_from(args)?.execute().await?;
        Ok(())
    }

    pub async fn add_proposal_to_allow_list(
        &self,
        proposal_id: u64,
        node_api_key: Option<String>,
    ) -> Result<()> {
        let proposal_id = format!("u64:{}", proposal_id);

        let mut args = vec![
            "",
            "--function-id",
            "0x1::velor_governance::add_approved_script_hash_script",
            "--sender-account",
            "0xa550c18",
            "--args",
            proposal_id.as_str(),
            "--private-key-file",
            self.root_key_path.as_path().to_str().unwrap(),
            "--assume-yes",
            "--encoding",
            "bcs",
            "--url",
            self.endpoint.as_str(),
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        RunFunction::try_parse_from(args)?.execute().await?;
        Ok(())
    }

    pub async fn execute_proposal(
        &self,
        proposal_id: u64,
        script_path: &Path,
        node_api_key: Option<String>,
    ) -> Result<()> {
        println!(
            "Executing: {:?} at proposal id {:?}",
            script_path, proposal_id
        );

        let address_string = format!("{}", self.validator_account);
        let privkey_string = self.get_hex_encoded_validator_key();
        let proposal_id = format!("{}", proposal_id);

        let mut args = vec![
            "",
            "--proposal-id",
            proposal_id.as_str(),
            "--script-path",
            script_path.to_str().unwrap(),
            "--sender-account",
            address_string.as_str(),
            "--private-key",
            privkey_string.as_str(),
            "--assume-yes",
            "--url",
            self.endpoint.as_str(),
            // Use the max gas unit for now. The simulate API sometimes cannot get the right gas estimate for proposals.
            "--max-gas",
            "2000000",
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        let rev = self.framework_git_rev.clone();
        let framework_path = velor_framework_path();
        if let Some(rev) = &rev {
            args.push("--framework-git-rev");
            args.push(rev.as_str());
        } else {
            args.push("--framework-local-dir");
            args.push(framework_path.as_os_str().to_str().unwrap());
        };

        ExecuteProposal::try_parse_from(args)?.execute().await?;
        Ok(())
    }

    async fn increase_lockup(&self, node_api_key: Option<String>) -> Result<()> {
        let validator_account = self.validator_account.to_string();
        let validator_key = self.get_hex_encoded_validator_key();
        let mut args = vec![
            // Ahhhhh this first empty string is very important
            // parse_from requires argv[0]
            "",
            "--sender-account",
            validator_account.as_str(),
            "--private-key",
            validator_key.as_str(),
            "--url",
            self.endpoint.as_str(),
            "--assume-yes",
        ];

        if let Some(api_key) = node_api_key.as_ref() {
            args.push("--node-api-key");
            args.push(api_key.as_str());
        }

        IncreaseLockup::try_parse_from(args)?.execute().await?;
        Ok(())
    }
}

async fn execute_release(
    release_config: ReleaseConfig,
    network_config: NetworkConfig,
    output_dir: Option<PathBuf>,
    validate_release: bool,
    node_api_key: Option<String>,
) -> Result<()> {
    let scripts_path = TempPath::new();
    scripts_path.create_as_dir()?;

    let proposal_folder = if let Some(dir) = &output_dir {
        dir.as_path()
    } else {
        scripts_path.path()
    };
    release_config
        .generate_release_proposal_scripts(proposal_folder)
        .await?;

    network_config.increase_lockup(node_api_key.clone()).await?;

    // Execute proposals
    for proposal in &release_config.proposals {
        let mut proposal_path = proposal_folder.to_path_buf();
        proposal_path.push("sources");
        proposal_path.push(&release_config.name);
        proposal_path.push(proposal.name.as_str());

        let mut script_paths: Vec<PathBuf> = std::fs::read_dir(proposal_path.as_path())?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().map(|s| s == "move").unwrap_or(false) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        script_paths.sort();

        match proposal.execution_mode {
            ExecutionMode::MultiStep => {
                network_config.set_fast_resolve(30).await?;
                network_config
                    .submit_and_execute_multi_step_proposal(
                        &proposal.metadata,
                        script_paths,
                        node_api_key.clone(),
                    )
                    .await?;

                network_config.set_fast_resolve(43200).await?;
            },
            ExecutionMode::RootSigner => {
                for entry in script_paths {
                    println!("Executing: {:?}", entry);
                    let mut args = vec![
                        "",
                        "--script-path",
                        entry.as_path().to_str().unwrap(),
                        "--sender-account",
                        "0xa550c18",
                        "--private-key-file",
                        network_config.root_key_path.as_path().to_str().unwrap(),
                        "--assume-yes",
                        "--encoding",
                        "bcs",
                        "--url",
                        network_config.endpoint.as_str(),
                    ];

                    if let Some(api_key) = node_api_key.as_ref() {
                        args.push("--node-api-key");
                        args.push(api_key.as_str());
                    }

                    let rev = network_config.framework_git_rev.clone();
                    let framework_path = velor_framework_path();
                    if let Some(rev) = &rev {
                        args.push("--framework-git-rev");
                        args.push(rev.as_str());
                    } else {
                        args.push("--framework-local-dir");
                        args.push(framework_path.as_os_str().to_str().unwrap());
                    };

                    RunScript::try_parse_from(args)?.execute().await?;
                }
            },
        };
        if validate_release {
            release_config
                .validate_upgrade(&network_config.endpoint, proposal)
                .await?;
        }
    }
    Ok(())
}

pub async fn validate_config(
    release_config: ReleaseConfig,
    network_config: NetworkConfig,
    node_api_key: Option<String>,
) -> Result<()> {
    validate_config_and_generate_release(release_config, network_config, None, node_api_key).await
}

pub async fn validate_config_and_generate_release(
    release_config: ReleaseConfig,
    network_config: NetworkConfig,
    output_dir: Option<PathBuf>,
    node_api_key: Option<String>,
) -> Result<()> {
    execute_release(
        release_config.clone(),
        network_config.clone(),
        output_dir,
        true,
        node_api_key,
    )
    .await
}

#[cfg(test)]
pub mod test {
    use super::NetworkConfig;
    use velor_crypto::PrivateKey;
    use velor_keygen::KeyGen;
    use velor_types::transaction::authenticator::AuthenticationKey;

    #[tokio::test]
    pub async fn test_network_config() {
        let seed_slice = [0u8; 32];
        let mut keygen = KeyGen::from_seed(seed_slice);
        let validator_key = keygen.generate_ed25519_private_key();
        let validator_account =
            AuthenticationKey::ed25519(&validator_key.public_key()).account_address();

        let network_info = NetworkConfig {
            endpoint: "https://banana.com/".parse().unwrap(),
            root_key_path: "".into(),
            validator_account,
            validator_key,
            framework_git_rev: None,
        };

        let private_key_string = network_info.get_hex_encoded_validator_key();
        assert_eq!(
            private_key_string.as_str(),
            "76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
        );
    }
}
