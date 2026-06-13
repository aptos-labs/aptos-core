// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_framework::extended_checks;
use move_binary_format::file_format_common;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::CompilerConfig;
use std::{process::Command, str::FromStr};

/// Optimization level during the compilation
#[derive(Copy, Clone)]
pub enum OptLevel {
    /// No optimizations
    None,
    /// Default optimization level
    Default,
    /// Extra optimizations, that may take more time
    Extra,
}

/// Move compilation specification
#[derive(Copy, Clone)]
pub struct LanguageSetting {
    pub version: LanguageVersion,
    pub optimization: OptLevel,
}

impl FromStr for LanguageSetting {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (optimization, rest) = match s.strip_suffix('-') {
            None => match s.strip_suffix('+') {
                None => (OptLevel::Default, s),
                Some(r) => (OptLevel::Extra, r),
            },
            Some(r) => (OptLevel::None, r),
        };
        let version = LanguageVersion::from_str(rest)?;

        // sanity check
        if matches!(version, LanguageVersion::V1) && !matches!(optimization, OptLevel::Default) {
            bail!("V1 of the language does not support optimization");
        }

        // done
        Ok(LanguageSetting {
            version,
            optimization,
        })
    }
}

impl LanguageSetting {
    fn compiler_version(version: LanguageVersion) -> CompilerVersion {
        match version {
            LanguageVersion::V1 => CompilerVersion::V1,
            LanguageVersion::V2_0 | LanguageVersion::V2_1 => CompilerVersion::V2_0,
            LanguageVersion::V2_2
            | LanguageVersion::V2_3
            | LanguageVersion::V2_4
            | LanguageVersion::V2_5 => CompilerVersion::V2_1,
        }
    }

    /// Derive a suitable `CompilerConfig` based on the language setting
    pub fn derive_compilation_config(&self) -> CompilerConfig {
        let Self {
            version,
            optimization,
        } = self;

        let mut experiments = vec![];
        if !matches!(version, LanguageVersion::V1) {
            match optimization {
                OptLevel::Default => {
                    experiments.push("optimize=on".to_string());
                },
                OptLevel::None => {
                    experiments.push("optimize=off".to_string());
                },
                OptLevel::Extra => {
                    experiments.push("optimize=on".to_string());
                    experiments.push("optimize-extra=on".to_string());
                },
            }
        }

        // FIXME(mengxu): keep in sync with `aptos_framework::build_package::BuildOptions::move_2()`
        CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            skip_attribute_checks: false,
            print_errors: Some(true),
            language_version: Some(*version),
            compiler_version: Some(Self::compiler_version(*version)),
            bytecode_version: Some(match version {
                LanguageVersion::V1 => file_format_common::VERSION_6,
                LanguageVersion::V2_0 => file_format_common::VERSION_7,
                LanguageVersion::V2_1 => file_format_common::VERSION_7,
                LanguageVersion::V2_2 => file_format_common::VERSION_8,
                LanguageVersion::V2_3 => file_format_common::VERSION_9,
                LanguageVersion::V2_4 | LanguageVersion::V2_5 => file_format_common::VERSION_10,
            }),
            experiments,
        }
    }

    /// Derive the suitable CLI options based on the language setting
    pub fn derive_cli_options(&self, command: &mut Command) {
        let Self {
            version,
            optimization,
        } = self;

        // FIXME(mengxu): keep in sync with `aptos_framework::build_package::BuildOptions::move_2()`
        match version {
            LanguageVersion::V1 => command.args([
                "--language-version",
                "1",
                "--compiler-version",
                "1",
                "--bytecode-version",
                "6",
            ]),
            LanguageVersion::V2_0 => command.args([
                "--language-version",
                "2.0",
                "--compiler-version",
                "2.0",
                "--bytecode-version",
                "7",
            ]),
            LanguageVersion::V2_1 => command.args([
                "--language-version",
                "2.1",
                "--compiler-version",
                "2.0",
                "--bytecode-version",
                "7",
            ]),
            LanguageVersion::V2_2 => command.args([
                "--language-version",
                "2.2",
                "--compiler-version",
                "2.1",
                "--bytecode-version",
                "8",
            ]),
            LanguageVersion::V2_3 => command.args([
                "--language-version",
                "2.3",
                "--compiler-version",
                "2.1",
                "--bytecode-version",
                "9",
            ]),
            LanguageVersion::V2_4 => command.args([
                "--language-version",
                "2.4",
                "--compiler-version",
                "2.1",
                "--bytecode-version",
                "10",
            ]),
            LanguageVersion::V2_5 => command.args([
                "--language-version",
                "2.5",
                "--compiler-version",
                "2.1",
                "--bytecode-version",
                "10",
            ]),
        };
        match optimization {
            OptLevel::None => command.args(["--optimize", "none"]),
            OptLevel::Default => command.args(["--optimize", "default"]),
            OptLevel::Extra => command.args(["--optimize", "extra"]),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{LanguageSetting, OptLevel};
    use anyhow::Result;
    use move_model::metadata::{CompilerVersion, LanguageVersion};
    use std::{process::Command, str::FromStr};

    fn command_args(setting: LanguageSetting) -> Vec<String> {
        let mut command = Command::new("aptos");
        setting.derive_cli_options(&mut command);
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn compiler_flag(args: &[String]) -> Option<&str> {
        args.windows(2)
            .find(|pair| pair[0] == "--compiler-version")
            .map(|pair| pair[1].as_str())
    }

    #[test]
    fn test_language_setting_from_str_parses_optimization_suffixes() -> Result<()> {
        let default = LanguageSetting::from_str("2.3")?;
        assert!(matches!(default.optimization, OptLevel::Default));

        let none = LanguageSetting::from_str("2.3-")?;
        assert!(matches!(none.optimization, OptLevel::None));

        let extra = LanguageSetting::from_str("2.3+")?;
        assert!(matches!(extra.optimization, OptLevel::Extra));
        Ok(())
    }

    #[test]
    fn test_language_setting_rejects_v1_non_default_optimization() {
        assert!(LanguageSetting::from_str("1-").is_err());
        assert!(LanguageSetting::from_str("1+").is_err());
    }

    #[test]
    fn test_language_setting_cli_and_config_use_same_compiler_version() {
        for version in [
            LanguageVersion::V1,
            LanguageVersion::V2_0,
            LanguageVersion::V2_1,
            LanguageVersion::V2_2,
            LanguageVersion::V2_3,
            LanguageVersion::V2_4,
            LanguageVersion::V2_5,
        ] {
            let setting = LanguageSetting {
                version,
                optimization: OptLevel::Default,
            };
            let config = setting.derive_compilation_config();
            let args = command_args(setting);
            let cli_compiler = compiler_flag(&args).unwrap();
            assert_eq!(
                cli_compiler.parse::<CompilerVersion>().unwrap(),
                config.compiler_version.unwrap(),
                "compiler-version mismatch for language {version}",
            );
        }
    }

    #[test]
    fn test_language_setting_v2_1_uses_stable_compiler_in_cli() {
        let args = command_args(LanguageSetting {
            version: LanguageVersion::V2_1,
            optimization: OptLevel::Default,
        });
        let expected = CompilerVersion::V2_0.to_string();
        assert_eq!(compiler_flag(&args), Some(expected.as_str()));
    }
}
