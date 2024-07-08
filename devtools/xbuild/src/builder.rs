use crate::flags;
use std::{
    ffi::OsString,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    str::FromStr,
    thread,
};
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

    cmd!(sh, "printenv").run().unwrap();

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
        let sh1 = sh.clone();
        let node_profile = self.node_profile.clone();
        let node_feature = self.node_feature.clone();
        let t1 = thread::spawn(move || {
            build(
                &sh1,
                vec!["aptos-node".into(), "aptos-forge-cli".into()],
                node_profile,
                node_feature,
            );
        });

        // let sh2 = sh.clone();
        // let node_profile = self.node_profile.clone();
        // let t2 = thread::spawn(move || {
        //     build(&sh2, vec!["aptos-forge-cli".into()], node_profile, None);
        // });

        let t3 = thread::spawn(move || build(&sh, vec!["aptos".into()], Some("cli".into()), None));

        t1.join().unwrap();
        // t2.join().unwrap();
        t3.join().unwrap();
    }
}
