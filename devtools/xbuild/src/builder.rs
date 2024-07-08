use crate::flags;
use std::{collections::BTreeMap, ffi::OsString, str::FromStr};
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

    sh.set_var(
        "CARGO_TARGET_DIR",
        format!(
            "target/{}/{}",
            feature.clone().into_string().unwrap(),
            profile.clone().into_string().unwrap(),
        ),
    );

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
        build_multiple(sh, vec![
            BuildTarget {
                package: "aptos-node".into(),
                feature: "".into(),
                profile: "release".into(),
            },
            BuildTarget {
                package: "aptos".into(),
                feature: "".into(),
                profile: "cli".into(),
            },
            BuildTarget {
                package: "aptos-forge-cli".into(),
                feature: "".into(),
                profile: "release".into(),
            },
        ])
    }
}

struct BuildTarget {
    package: OsString,
    feature: OsString,
    profile: OsString,
}

fn build_multiple(sh: Shell, targets: Vec<BuildTarget>) {
    let mut targets_by_profile_feature = BTreeMap::new();
    for target in targets.into_iter() {
        targets_by_profile_feature
            .entry(target.profile)
            .or_insert_with(|| BTreeMap::new())
            .entry(target.feature)
            .or_insert_with(|| Vec::new())
            .push(target.package);
    }

    let commands: Vec<_> = targets_by_profile_feature
        .into_iter()
        .flat_map(|(profile, targets_by_feature)| {
            targets_by_feature
                .into_iter()
                .map(move |(feature, targets)| (profile.clone(), feature, targets))
        })
        .map(|(profile, feature, targets)| {
            let sh = sh.clone();
            std::thread::spawn(move || build(&sh, targets, Some(profile), Some(feature)))
        })
        .collect();

    for command in commands {
        let _ = command.join();
    }
}
