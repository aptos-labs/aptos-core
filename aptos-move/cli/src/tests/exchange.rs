// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::exchange;
use serde_json::json;

#[test]
fn count_down() {
    let json = serde_json::to_value(
        exchange::masm_to_module(
            r#"
module 0x42::count_down

fun count_down(x: u64): u64
l1: copy_loc x
    ld_u64 0
    gt
    br_false l2
    copy_loc x
    ld_u64 1
    sub
    st_loc x
    branch l1
l2: move_loc x
    ret
"#,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(json["version"], json!(5));
    let fun = &json["funs"][0];
    assert_eq!(fun["name"], json!("count_down"));
    assert_eq!(fun["params"], json!(1));
    // All temporaries are declared with their types; one return.
    assert_eq!(fun["locals"][0], json!("u64"));
    assert_eq!(fun["returns"], json!(["u64"]));
    // Three blocks: header (gt became lt with swapped operands), body with
    // a back edge to the header, exit.
    let blocks = fun["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0]["instrs"][2], json!({"call": [[3], "lt", [2, 1]]}));
    assert_eq!(blocks[0]["term"], json!({"branch": [3, 1, 2]}));
    assert_eq!(blocks[1]["term"], json!({"jump": 0}));
    assert_eq!(blocks[2]["term"], json!({"ret": [7]}));
    // One natural loop with the header and body as members.
    let lp = &fun["loops"][0];
    assert_eq!(lp["header"], json!(0));
    assert_eq!(lp["members"], json!([0, 1]));
}

#[test]
fn account() {
    let json = serde_json::to_value(
        exchange::masm_to_module(
            r#"
module 0x42::account

struct Account has key
  balance: u64

fun withdraw(s: &signer, addr: address, amount: u64) acquires Account
    local tmp: Account
    copy_loc addr
    move_from Account
    unpack Account
    copy_loc amount
    sub
    pack Account
    st_loc tmp
    copy_loc s
    move_loc tmp
    move_to Account
    ret
"#,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        json["structs"],
        json!([{"name": "Account", "fields": [{"name": "balance", "ty": "u64"}]}])
    );
    // Parameter types include the reference: `s: &signer`.
    assert_eq!(json["funs"][0]["locals"][0], json!({"ref": "signer"}));
    assert_eq!(json["funs"][0]["locals"][1], json!("address"));
    let fun = &json["funs"][0];
    let instrs = fun["blocks"][0]["instrs"].as_array().unwrap();
    assert_eq!(instrs[1], json!({"call": [[5], {"move_from": 0}, [4]]}));
    assert_eq!(instrs[2], json!({"call": [[6], "unpack", [5]]}));
    assert_eq!(instrs[9], json!({"call": [[], {"move_to": 0}, [10, 11]]}));
}

#[test]
fn unsupported_rejected() {
    let err = exchange::masm_to_module(
        r#"
module 0x42::m

fun f(): u8
    ld_u8 1
    ret
"#,
    )
    .unwrap_err();
    assert!(format!("{:#}", err).contains("unsupported type"));
}

#[test]
fn specs() {
    let json = serde_json::to_value(
        exchange::masm_to_module(
            r#"
module 0x42::count_down

fun count_down(x: u64): u64
    requires x < 1000
    ensures result == 0
    ensures forall y: u64 . 0 <= y
    invariant l1: x < 1000
l1: copy_loc x
    ld_u64 0
    gt
    br_false l2
    copy_loc x
    ld_u64 1
    sub
    st_loc x
    branch l1
l2: move_loc x
    ret
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let fun = &json["funs"][0];
    assert_eq!(
        fun["spec"]["requires"],
        json!([{"binop": ["lt", {"local": 0}, {"value": {"u64": "1000"}}]}])
    );
    assert_eq!(
        fun["spec"]["ensures"],
        json!([{"binop": ["eq", {"result": 0}, {"value": {"u64": "0"}}]},
               {"quant": ["all", "u64",
                          {"binop": ["le", {"value": {"u64": "0"}}, {"bvar": 0}]}]}])
    );
    assert_eq!(
        fun["loops"][0]["invariants"],
        json!([{"binop": ["lt", {"local": 0}, {"value": {"u64": "1000"}}]}])
    );
}

#[test]
fn move_source() {
    let json = serde_json::to_value(
        exchange::move_source_to_module(
            r#"
module 0x42::account {
    struct Account has key { balance: u64 }

    fun take(addr: address): u64 acquires Account {
        let Account { balance } = move_from<Account>(addr);
        balance
    }
    spec take {
        aborts_if !exists<Account>(addr);
        ensures result == old(global<Account>(addr).balance);
        modifies global<Account>(addr);
    }
}
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let fun = &json["funs"][0];
    // No synthesized typing assumptions: consumers derive well-formedness
    // from the declared types.
    assert_eq!(fun["spec"]["requires"], json!([]));
    assert_eq!(
        fun["spec"]["aborts_if"][0],
        json!({"not": {"exists": [0, null, {"local": 0}]}})
    );
    // In a post-state clause, `old(..)` becomes snapshot label 0.
    assert_eq!(
        fun["spec"]["ensures"][0],
        json!({"binop": ["eq", {"result": 0},
                         {"select": [0, {"global": [0, 0, {"local": 0}]}]}]})
    );
    assert_eq!(
        fun["spec"]["modifies"],
        json!([{"resource": 0, "addr": {"local": 0}}])
    );
}

#[test]
fn move_source_skips_non_executable_declarations() {
    let json = serde_json::to_value(
        exchange::move_source_to_module(
            r#"
module 0x42::declarations {
    inline fun increment(x: u64): u64 {
        x + 1
    }

    spec lemma reflexive(x: u64) {
        ensures x == x;
    }

    fun call_inline(x: u64): u64 {
        increment(x)
    }
}
"#,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(json["funs"].as_array().unwrap().len(), 1);
    assert_eq!(json["funs"][0]["name"], json!("call_inline"));
}

#[test]
fn move_source_rejects_ghost_field_specs() {
    let err = exchange::move_source_to_module(
        r#"
module 0x42::ghost_fields {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost g: u64;
    }

    fun identity(s: S): S {
        s
    }
    spec identity {
        ensures result.g == s.g;
    }
}
"#,
    )
    .unwrap_err();
    let err = format!("{err:#}");
    assert!(err.contains("ghost field `g` not supported"), "{err}");
}

#[test]
fn move_source_rejects_old_in_aborts_if() {
    let err = exchange::move_source_to_module(
        r#"
module 0x42::abort_state {
    struct R has key { value: u64 }

    fun f(addr: address) {
    }
    spec f {
        aborts_if old(exists<R>(addr));
    }
}
"#,
    )
    .unwrap_err();
    let err = format!("{err:#}");
    assert!(
        err.contains("`old(..)` expression not allowed in this context"),
        "{err}"
    );
}

#[test]
fn move_source_loop_invariant() {
    let json = serde_json::to_value(
        exchange::move_source_to_module(
            r#"
module 0x42::count_down {
    fun count_down(x: u64): u64 {
        while (0 < x) {
            x = x - 1;
        } spec {
            invariant x <= 18446744073709551615;
        };
        x
    }
    spec count_down {
        ensures result == 0;
    }
}
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let fun = &json["funs"][0];
    assert_eq!(fun["loops"][0]["header"], json!(0));
    assert_eq!(
        fun["loops"][0]["invariants"],
        json!([{"binop": ["le", {"local": 0},
                         {"value": {"u64": "18446744073709551615"}}]}])
    );
}
