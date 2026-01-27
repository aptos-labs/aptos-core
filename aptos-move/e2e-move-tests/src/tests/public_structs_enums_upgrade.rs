// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for upgrade compatibility of public/package structs/enums

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn public_enum_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V1{x: u64}
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            public enum Data {
               V1{x: u64},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            public enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            package enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[test]
fn friend_enum_struct_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            friend 0x815::m2;
            enum Data {
               V1{x: u64}
            }
        }
        module 0x815::m2 {
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
            use 0x815::m::Data;
            fun get_data(): Data {
                Data::V2{x: 1, y: 2}
            }
        }
        module 0x815::m {
            friend 0x815::m2;
            friend enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
        }
        module 0x815::m {
            enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
            use 0x815::m::Data;
            use 0x815::m::S;
            fun get_data(): Data {
                Data::V1{x: 1}
            }
            fun pack_struct(): S {
                S { x: 22 }
            }
        }
        module 0x815::m {
            package enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }

            package struct S {
                x: u64,
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
            module 0x815::m2 {
                use 0x815::m::Data;
                use 0x815::m::S;
                use 0x815::m::Predicate;
                use 0x815::m::Holder;
                fun get_data(): Data {
                    Data::V1{x: 1}
                }
                public fun pack_struct(): S {
                S { x: 22 }
                }

                public fun apply_pred(p: Predicate<u64>, val: u64): bool {
                    p(&val)
                }

                public fun apply_holder(h: Holder<u64>, val: u64): bool {
                    let Holder(p) = h;
                    p(&val)
                }
            }
            module 0x815::m {
                public enum Data {
                   V1{x: u64},
                   V2{x: u64, y: u8},
                }

                public struct S {
                    x: u64,
                }

                package struct Predicate<T>(|&T| bool) has copy, drop;

                package struct Holder<T>(Predicate<T>) has copy, drop;
            }
        "#,
    );
    assert_success!(result);
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}
