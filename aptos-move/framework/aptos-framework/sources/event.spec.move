spec aptos_framework::event {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec emit_event {
        pragma opaque;
        aborts_if [abstract] false;
        ensures handle_ref.counter < MAX_U64 ==> handle_ref.counter == old(handle_ref.counter) + 1;
    }

    /// Native function use opaque.
    spec write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T) {
        pragma opaque;
    }

    spec guid {
        aborts_if false;
    }

    spec counter {
        aborts_if false;
    }

    spec destroy_handle {
        aborts_if false;
    }

}
