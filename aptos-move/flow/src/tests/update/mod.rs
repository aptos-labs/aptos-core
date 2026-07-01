// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::update::{run, strip_prerelease, UpdateArgs};
use clap::Parser;
use self_update::version::bump_is_greater;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: UpdateArgs,
}

#[test]
fn strip_prerelease_suffixes() {
    assert_eq!(strip_prerelease("1.0.5.beta"), "1.0.5");
    assert_eq!(strip_prerelease("1.0.5.rc"), "1.0.5");
    assert_eq!(strip_prerelease("1.0.5"), "1.0.5");
    assert_eq!(strip_prerelease("1.0.5.other"), "1.0.5.other");
}

#[test]
fn tag_version_extraction() {
    let tag = "move-flow-v1.0.5";
    assert_eq!(&tag["move-flow-v".len()..], "1.0.5");
}

#[test]
fn version_comparison() {
    assert!(bump_is_greater("1.0.4", "1.0.5").unwrap());
    assert!(!bump_is_greater("1.0.5", "1.0.4").unwrap());
    assert!(!bump_is_greater("1.0.5", "1.0.5").unwrap());
    assert!(bump_is_greater("1.0.4", strip_prerelease("1.0.5.beta")).unwrap());
}

#[test]
fn check_flag_parses() {
    let cli = Cli::try_parse_from(["cmd", "--check"]).unwrap();
    assert!(cli.args.check);
}

#[test]
fn check_flag_defaults_false() {
    let cli = Cli::try_parse_from(["cmd"]).unwrap();
    assert!(!cli.args.check);
}

#[test]
#[ignore]
fn live_github_check() {
    let args = UpdateArgs {
        check: true,
        repo_owner: "aptos-labs".into(),
        repo_name: "aptos-ai".into(),
    };
    let result = run(&args);
    assert!(result.is_ok(), "live check failed: {result:?}");
}
