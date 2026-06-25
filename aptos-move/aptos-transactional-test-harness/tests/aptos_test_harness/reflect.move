// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key Alice
module Alice::emit_bypass {
    use std::reflect;
    use std::string;

    struct NotAnEvent has copy, store, drop { value: u64 }

    #[event]
    struct OwnEvent has copy, store, drop { value: u64 }

    public entry fun emit_non_event_rejected() {
        let err = reflect::resolve<|NotAnEvent|>(
            @0x1,
            &string::utf8(b"event"),
            &string::utf8(b"emit"),
        ).unwrap_err();
        assert!(err.error_code() == 2, 0xBAD);
    }

    public entry fun emit_own_event_rejected() {
        let err = reflect::resolve<|OwnEvent|>(
            @0x1,
            &string::utf8(b"event"),
            &string::utf8(b"emit"),
        ).unwrap_err();
        assert!(err.error_code() == 2, 0xBAD);
    }
}

//# run --signers Alice -- Alice::emit_bypass::emit_non_event_rejected

//# run --signers Alice -- Alice::emit_bypass::emit_own_event_rejected

//# publish --private-key Alice
module Alice::victim {
    public struct SecretData has copy, store, drop { data: u64 }

    public fun with_data(callback: |SecretData|) {
        callback(SecretData { data: 9999 })
    }
}

//# publish --private-key Alice
module Alice::attacker {
    use std::reflect;
    use std::string;
    use Alice::victim::SecretData;

    public entry fun cross_module_emit_rejected() {
        let err = reflect::resolve<|SecretData|>(
            @0x1,
            &string::utf8(b"event"),
            &string::utf8(b"emit"),
        ).unwrap_err();
        assert!(err.error_code() == 2, 0xBAD);
    }
}

//# run --signers Alice -- Alice::attacker::cross_module_emit_rejected
