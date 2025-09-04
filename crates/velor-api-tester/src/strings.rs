// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Fail messages

pub const FAIL_WRONG_ACCOUNT_DATA: &str = "wrong account data";
pub const FAIL_WRONG_BALANCE: &str = "wrong balance";
pub const FAIL_WRONG_BALANCE_AT_VERSION: &str = "wrong balance at version";
pub const FAIL_WRONG_COLLECTION_DATA: &str = "wrong collection data";
pub const FAIL_WRONG_MESSAGE: &str = "wrong message";
pub const FAIL_WRONG_MODULE: &str = "wrong module";
pub const FAIL_WRONG_TOKEN_BALANCE: &str = "wrong token balance";
pub const FAIL_WRONG_TOKEN_DATA: &str = "wrong token data";

// Error messages

pub const ERROR_BAD_BALANCE_STRING: &str = "bad balance string";
pub const ERROR_COULD_NOT_BUILD_PACKAGE: &str = "failed to build package";
pub const ERROR_COULD_NOT_CHECK: &str = "persistency check never started";
pub const ERROR_COULD_NOT_CREATE_ACCOUNT: &str = "failed to create account";
pub const ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION: &str =
    "failed to create and submit transaction";
pub const ERROR_COULD_NOT_FINISH_TRANSACTION: &str = "failed to finish transaction";
pub const ERROR_COULD_NOT_FUND_ACCOUNT: &str = "failed to fund account";
pub const ERROR_COULD_NOT_SERIALIZE: &str = "failed to serialize";
pub const ERROR_COULD_NOT_VIEW: &str = "view function failed";
pub const ERROR_NO_ACCOUNT_DATA: &str = "can't find account data";
pub const ERROR_NO_BALANCE: &str = "can't find account balance";
pub const ERROR_NO_BALANCE_STRING: &str = "the API did not return a balance string";
pub const ERROR_NO_BYTECODE: &str = "can't find bytecode";
pub const ERROR_NO_COLLECTION_DATA: &str = "can't find collection data";
pub const ERROR_NO_MESSAGE: &str = "can't find message";
pub const ERROR_NO_METADATA: &str = "can't find metadata";
pub const ERROR_NO_MODULE: &str = "can't find module";
pub const ERROR_NO_TOKEN_BALANCE: &str = "can't find token balance";
pub const ERROR_NO_TOKEN_DATA: &str = "can't find token data";
pub const ERROR_NO_VERSION: &str = "can't find transaction version";

// Step names

pub const SETUP: &str = "setup";
pub const CHECK_ACCOUNT_DATA: &str = "check_account_data";
pub const FUND: &str = "fund";
pub const CHECK_ACCOUNT_BALANCE: &str = "check_account_balance";
pub const TRANSFER_COINS: &str = "transfer_coins";
pub const CHECK_ACCOUNT_BALANCE_AT_VERSION: &str = "check_account_balance_at_version";
pub const CREATE_COLLECTION: &str = "create_collection";
pub const CHECK_COLLECTION_METADATA: &str = "check_collection_metadata";
pub const CREATE_TOKEN: &str = "create_token";
pub const CHECK_TOKEN_METADATA: &str = "check_token_metadata";
pub const CHECK_SENDER_BALANCE: &str = "check_sender_balance";
pub const OFFER_TOKEN: &str = "offer_token";
pub const CLAIM_TOKEN: &str = "claim_token";
pub const CHECK_RECEIVER_BALANCE: &str = "check_receiver_balance";
pub const BUILD_MODULE: &str = "build_module";
pub const PUBLISH_MODULE: &str = "publish_module";
pub const CHECK_MODULE_DATA: &str = "check_module_data";
pub const SET_MESSAGE: &str = "set_message";
pub const CHECK_MESSAGE: &str = "check_message";
pub const CHECK_VIEW_ACCOUNT_BALANCE: &str = "check_view_account_balance";
