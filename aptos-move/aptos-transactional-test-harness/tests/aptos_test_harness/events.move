// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

// Tests that emitting events with a type-tag depth exceeding the limit (7 nested vectors) is
// rejected with ECANNOT_CREATE_EVENT, for both V1 (EventHandle) and V2 (#[event]) events.

//# publish --private-key Alice
module Alice::events_test {
    use aptos_framework::account;
    use aptos_framework::event;

    struct V1Event<phantom T> has store, drop {}

    public entry fun emit_v1_ok(account: &signer) {
        let stream = account::new_event_handle<V1Event<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>(account);
        event::emit_event(&mut stream, V1Event<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>{});
        event::destroy_handle(stream);
    }

    public entry fun emit_v1_too_large(account: &signer) {
        let stream = account::new_event_handle<V1Event<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>>(account);
        event::emit_event(&mut stream, V1Event<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>{});
        event::destroy_handle(stream); // never reached
    }

    #[event]
    struct V2Event<phantom T> has store, drop {}

    public entry fun emit_v2_ok(_: &signer) {
        event::emit(V2Event<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>{});
    }

    public entry fun emit_v2_too_large(_: &signer) {
        event::emit(V2Event<vector<vector<vector<vector<vector<vector<vector<vector<u8>>>>>>>>>{});
    }
}

//# run --signers Alice -- Alice::events_test::emit_v1_ok

//# run --signers Alice -- Alice::events_test::emit_v1_too_large

//# run --signers Alice -- Alice::events_test::emit_v2_ok

//# run --signers Alice -- Alice::events_test::emit_v2_too_large
