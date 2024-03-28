/// test_point1: overflows the 90 column limit
/// test_point2: merge itmes

module BlockLine {
    use aptos_framework::{
        account::{
            Self,
            SignerCapability,
            Account,
            create_signer_with_capability,
            new_event_handle
        },
        coin::{
            Self,
            BurnCapability,
            MintCapability
        }
    };
}
