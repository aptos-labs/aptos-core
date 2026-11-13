// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::update::{run, UpdateArgs};
use clap::Parser;
use self_update::version::bump_is_greater;

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: UpdateArgs,
}

#[test]
fn version_comparison() {
    assert!(bump_is_greater("1.0.4", "1.0.5").unwrap());
    assert!(!bump_is_greater("1.0.5", "1.0.4").unwrap());
    assert!(!bump_is_greater("1.0.5", "1.0.5").unwrap());
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
