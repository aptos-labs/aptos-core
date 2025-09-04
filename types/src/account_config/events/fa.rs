use derive_getters::Getters;
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag, move_resource::MoveStructType};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static FA_WITHDRAW_EVENT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(FaWithdraw::struct_tag())));
pub static FA_DEPOSIT_EVENT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(FaDeposit::struct_tag())));

/// Represents a Deposit event for a Fungible Asset.
#[derive(Debug, Serialize, Deserialize, Getters)]
pub struct FaDeposit {
    store: AccountAddress,
    amount: u64,
}

/// Represents a Withdraw event for a Fungible Asset.
#[derive(Debug, Serialize, Deserialize, Getters)]
pub struct FaWithdraw {
    store: AccountAddress,
    amount: u64,
}

impl MoveStructType for FaDeposit {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Deposit");
}

impl MoveStructType for FaWithdraw {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Withdraw");
}
