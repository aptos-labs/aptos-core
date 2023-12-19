/// test_point: A list consisting of multiple items, with comments before the items

module test_use {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_std::type_info::{ /* use_item before */ Self, TypeInfo};
    use econia::resource_account;
    use econia::tablist::{Self, /* use_item before */ Tablist};
    use std::signer::address_of;
    use std::vector;
}