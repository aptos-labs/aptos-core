spec aptos_framework::event {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec emit_event {
        pragma opaque;
        aborts_if [abstract] false;
    }

    /// Native function use opaque.
    spec write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T) {
        pragma opaque;
    }
}
