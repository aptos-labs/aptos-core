spec aptos_framework::guid {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create(addr: address, creation_num_ref: &mut u64): GUID {
        aborts_if creation_num_ref + 1 > MAX_U64;
        ensures result.id.creation_num == old(creation_num_ref);
        ensures creation_num_ref == old(creation_num_ref) + 1;
    }
}
