spec aptos_framework::guid {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec id(guid: &GUID): ID {
        aborts_if false;
    }

    spec creator_address(guid: &GUID): address {
        aborts_if false;
    }

    spec id_creator_address(id: &ID): address {
        aborts_if false;
    }

    spec creation_num(guid: &GUID): u64 {
        aborts_if false;
    }

    spec id_creation_num(id: &ID): u64 {
        aborts_if false;
    }

    spec eq_id(guid: &GUID, id: &ID): bool {
        aborts_if false;
    }

    spec create(addr: address, creation_num_ref: &mut u64): GUID {
        aborts_if creation_num_ref + 1 > MAX_U64;
        ensures result.id.creation_num == old(creation_num_ref);
        ensures creation_num_ref == old(creation_num_ref) + 1;
    }
}
