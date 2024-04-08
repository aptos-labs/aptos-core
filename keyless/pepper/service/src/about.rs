// Copyright © Aptos Foundation

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AboutInfo {
    git_commit: String,
}

pub static ABOUT_JSON: Lazy<String> = Lazy::new(|| {
    let obj = AboutInfo {
        git_commit: std::env::var("GIT_COMMIT").unwrap_or_default(),
    };
    serde_json::to_string_pretty(&obj).unwrap()
});
