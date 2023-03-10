// todo authenticate or authorized
module aptos_framework::authenticate {
    use std::signer::address_of;
    use aptos_framework::create_signer;

    /// Successor to the signer type. A flexible and granular framework
    /// for managing proper authentication for operations.
    struct Auth<Op: copy + drop> has drop {
        acct: address,
        op: Op,
    }

    struct AuthenticationCapability<phantom Op: copy + drop> has key, store {
        acct: address,
    }

    struct CreateSigner has copy, drop {}

    public fun get_auth_from_signer<Op: copy + drop>(op: Op, s: &signer): Auth<Op> {
        Auth { acct: address_of(s), op}
    }

    public fun get_auth_from_signature<Op: copy + drop>(acct: address, op: Op, _version: u32, _salt: u256, _signature: &vector<u8>): Auth<Op> {
        // VerifySign(chainid, typename<Op>(), version, salt, acct, serialized(op)) == signature
        Auth { acct, op}
    }

    public fun get_auth_with_capability<Op: copy + drop>(op: Op, cap: &AuthenticationCapability<Op>): Auth<Op> {
        Auth { acct: cap.acct, op }
    }

    public fun get_signer(cap: &AuthenticationCapability<CreateSigner>): signer {
        create_signer(cap.acct);
    }

    public fun get_create_signer_capability(s: &signer): AuthenticationCapability<CreateSigner> {
        AuthenticationCapability { acct: address_of(s) }
    }

    public fun get_operation<Op: copy + drop>(auth: &Auth<Op>): Op {
        auth.op
    }
}
