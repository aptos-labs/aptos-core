use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag, move_resource::MoveStructType, parser::parse_type_tag
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use derive_getters::Getters;
pub static COIN_WITHDRAW_EVENT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(CoinWithdraw::struct_tag())));
pub static COIN_DEPOSIT_EVENT_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(CoinDeposit::struct_tag())));

pub static COIN_LEGACY_DEPOSIT_EVENT_V1: Lazy<TypeTag> = Lazy::new(|| {
    parse_type_tag("0x1::coin::DepositEvent")
        .expect("parse type tag for 0x1::coin::DepositEvent should succeed")
});
pub static COIN_LEGACY_WITHDRAW_EVENT_V1: Lazy<TypeTag> = Lazy::new(|| {
    parse_type_tag("0x1::coin::WithdrawEvent")
        .expect("parse type tag for 0x1::coin::WithdrawEvent should succeed")
});

/// Module event emitted when some amount of a coin is deposited into an account.
#[derive(Debug, Serialize, Deserialize, Getters)]
pub struct CoinDeposit {
    coin_type: String,
    account: AccountAddress,
    amount: u64,
}

/// Module event emitted when some amount of a coin is withdrawn from an account.
#[derive(Debug, Serialize, Deserialize, Getters)]
pub struct CoinWithdraw {
    coin_type: String,
    account: AccountAddress,
    amount: u64,
}

impl MoveStructType for CoinWithdraw {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinWithdraw");
}

impl MoveStructType for CoinDeposit {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinDeposit");
}