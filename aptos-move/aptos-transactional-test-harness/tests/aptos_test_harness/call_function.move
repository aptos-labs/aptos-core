//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

// TODO: Re-activate after rewriting publishing logic!

// Original outputs:
// processed 4 tasks
//
// task 2 'run'. lines 33-33:
// Events:
// {
// type:    0x1::transaction_fee::FeeStatement
// data:    "04000000000000000300000000000000020000000000000000000000000000000000000000000000"
// }mutable inputs after call: local#0: 0
// return values: 0
//
// task 3 'view'. lines 35-35:
// store key 0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6::hello_world::ModuleData {
// global_counter: 0
// state: copy drop store 0x1::string::String {
// bytes: 68656c6c6f20776f726c64
// }
// }

// //# publish
// module Alice::hello_world {
//     use aptos_framework::signer;
//     use aptos_framework::coin;
//     use aptos_framework::aptos_coin::AptosCoin;
//     use std::string::{Self, String};
//
//     struct ModuleData has key, store {
//         global_counter: u64,
//         state: String,
//     }
//
//     fun init_module(sender: &signer) {
//         move_to(
//             sender,
//             ModuleData { global_counter: 0, state: string::utf8(b"init") }
//         );
//     }
//
//     public fun foo(addr: address): u64 {
//         coin::balance<AptosCoin>(addr)
//     }
//
//     public entry fun hi(sender: &signer, msg: String) acquires ModuleData {
//         borrow_global_mut<ModuleData>(signer::address_of(sender)).state = msg;
//     }
// }
//
//
// //# run --signers Alice --args x"68656C6C6F20776F726C64" --show-events -- Alice::hello_world::hi
//
// //# view --address Alice --resource Alice::hello_world::ModuleData
