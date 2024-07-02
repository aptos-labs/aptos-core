use crate::flags;
use std::{ffi::OsString, str::FromStr};
use xshell::{cmd, Shell};

pub(crate) fn build(
    sh: &Shell,
    packages: Vec<OsString>,
    profile: Option<OsString>,
    feature: Option<OsString>,
) {
    let profile = profile.unwrap_or(OsString::from_str("release").unwrap());
    let feature = feature.unwrap_or(OsString::from_str("").unwrap());
    let packages: Vec<OsString> = packages
        .into_iter()
        .map(|package| vec!["-p".into(), package])
        .flatten()
        .collect();

    cmd!(
        sh,
        "cargo build {packages...} --profile={profile} --features={feature}"
    )
    .run()
    .unwrap()
}

impl flags::Node {
    pub(crate) fn run(self, sh: Shell) {
        build(&sh, vec!["aptos-node".into()], self.profile, self.feature)
    }
}

impl flags::Tools {
    pub(crate) fn run(self, sh: Shell) {
        build(&sh, self.packages, self.profile, self.feature)
    }
}

impl flags::Indexer {
    pub(crate) fn run(self, sh: Shell) {
        build(&sh, self.packages, self.profile, self.feature)
    }
}

impl flags::Group {
    pub(crate) fn run(self, sh: Shell) {
        match self.subcommand {
            flags::GroupCmd::Forge(forge) => forge.run(sh),
        }
    }
}

impl flags::Forge {
    pub(crate) fn run(self, sh: Shell) {
        build(
            &sh,
            vec!["aptos-node".into()],
            self.node_profile.clone(),
            self.node_feature,
        );
        build(&sh, vec!["aptos-forge-cli".into()], self.node_profile, None);

        build(&sh, vec!["aptos".into()], Some("ci".into()), None)
    }
}
