spec aptos_std::event {
    spec emit_event {
        pragma opaque;
        aborts_if [abstract] false;
    }

    spec module {
        pragma verify = false;
    }
}
