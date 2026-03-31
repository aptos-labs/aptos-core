// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(debug_assertions)]
pub const MOVEY_URL: &str = "https://movey-app-staging.herokuapp.com";
#[cfg(not(debug_assertions))]
pub const MOVEY_URL: &str = "https://www.movey.net";
pub const MOVEY_CREDENTIAL_PATH: &str = "/movey_credential.toml";
