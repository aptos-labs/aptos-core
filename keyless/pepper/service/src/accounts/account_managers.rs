// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    env,
};

/// If a pepper request v1 comes with a JWT from one of the privileged auds,
/// aud overriding is allowed.
///
/// ## How to use
/// For an account manager identified by issuer `<some-issuer>` and client ID `<some-aud>`,
/// give it a unique short name `<some-short-name>` and put the following as the environment variables.
/// `ACCOUNT_MANAGER_<some-short-name>_ISSUER=<some-issuer>
/// `ACCOUNT_MANAGER_<some-short-name>_AUD=<some-aud>
///
/// Multiple account managers can be specified, as long as they each have a unique short name.
///
/// Here is an example command.
/// ```bash
/// ACCOUNT_MANAGER_1A_ISSUER=https://accounts.google.com \
/// ACCOUNT_MANAGER_1A_AUD=1234567890 \
/// ACCOUNT_MANAGER_2B_ISSUER=https://accounts.facebook.com \
/// ACCOUNT_MANAGER_2B_AUD=9876543210 \
/// VUF_KEY_SEED_HEX=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
/// cargo run -p aptos-keyless-pepper-service
/// ```
pub static ACCOUNT_MANAGERS: Lazy<HashSet<(String, String)>> = Lazy::new(|| {
    let re_issuer = Regex::new(r"ACCOUNT_MANAGER_(\w+)_ISSUER").unwrap();
    let re_aud = Regex::new(r"ACCOUNT_MANAGER_(\w+)_AUD").unwrap();
    let mut working_set: HashMap<String, Collector> = HashMap::new();

    env::vars().for_each(|(key, val)| {
        if let Some(caps) = re_issuer.captures(key.as_str()) {
            let uid = caps.get(1).unwrap().as_str().to_string();
            working_set.entry(uid).or_default().set_issuer(val);
        } else if let Some(caps) = re_aud.captures(key.as_str()) {
            let uid = caps.get(1).unwrap().as_str().to_string();
            working_set.entry(uid).or_default().set_aud(val);
        }
    });

    let ret: HashSet<(String, String)> = working_set
        .values()
        .cloned()
        .filter_map(|collector| {
            let Collector { issuer, aud } = collector;
            if let (Some(issuer), Some(aud)) = (issuer, aud) {
                Some((issuer, aud))
            } else {
                None
            }
        })
        .collect();
    info!("ACCOUNT_MANAGERS={:?}", ret);
    ret
});

#[derive(Clone, Default)]
struct Collector {
    pub issuer: Option<String>,
    pub aud: Option<String>,
}

impl Collector {
    fn set_issuer(&mut self, issuer: String) {
        self.issuer = Some(issuer);
    }

    fn set_aud(&mut self, aud: String) {
        self.aud = Some(aud);
    }
}
