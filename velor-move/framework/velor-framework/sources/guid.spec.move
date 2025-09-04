spec velor_framework::guid {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The creation of GUID constructs a unique GUID by combining an address with an incremented creation
    /// number.
    /// Criticality: Low
    /// Implementation: The create function generates a new GUID by combining an address with an incremented creation
    /// number, effectively creating a unique identifier.
    /// Enforcement: Enforced via [high-level-req-1](create).
    ///
    /// No.: 2
    /// Requirement: The operations on GUID and ID, such as construction, field access, and equality comparison, should not
    /// abort.
    /// Criticality: Low
    /// Implementation: The following functions will never abort: (1) create_id, (2) id, (3) creator_address, (4)
    /// id_creator_address, (5) creation_num, (6) id_creation_num, and (7) eq_id.
    /// Enforcement: Verified via [high-level-req-2.1](create_id), [high-level-req-2.2](id), [high-level-req-2.3](creator_address), [high-level-req-2.4](id_creator_address), [high-level-req-2.5](creation_num), [high-level-req-2.6](id_creation_num), and [high-level-req-2.7](eq_id).
    ///
    /// No.: 3
    /// Requirement: The creation number should increment by 1 with each new creation.
    /// Criticality: Low
    /// Implementation: An account can only own up to MAX_U64 resources. Not incrementing the guid_creation_num
    /// constantly could lead to shrinking the space for new resources.
    /// Enforcement: Enforced via [high-level-req-3](create).
    ///
    /// No.: 4
    /// Requirement: The creation number and address of an ID / GUID must be immutable.
    /// Criticality: Medium
    /// Implementation: The addr and creation_num values are meant to be constant and never updated as they are unique
    /// and used for identification.
    /// Enforcement: Audited: This is enforced through missing functionality to update the creation_num or addr.
    /// </high-level-req>
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create_id(addr: address, creation_num: u64): ID {
        /// [high-level-req-2.1]
        aborts_if false;
    }

    spec id(guid: &GUID): ID {
        /// [high-level-req-2.2]
        aborts_if false;
    }

    spec creator_address(guid: &GUID): address {
        /// [high-level-req-2.3]
        aborts_if false;
    }

    spec id_creator_address(id: &ID): address {
        /// [high-level-req-2.4]
        aborts_if false;
    }

    spec creation_num(guid: &GUID): u64 {
        /// [high-level-req-2.5]
        aborts_if false;
    }

    spec id_creation_num(id: &ID): u64 {
        /// [high-level-req-2.6]
        aborts_if false;
    }

    spec eq_id(guid: &GUID, id: &ID): bool {
        /// [high-level-req-2.7]
        aborts_if false;
    }

    spec create(addr: address, creation_num_ref: &mut u64): GUID {
        aborts_if creation_num_ref + 1 > MAX_U64;
        /// [high-level-req-1]
        ensures result.id.creation_num == old(creation_num_ref);
        /// [high-level-req-3]
        ensures creation_num_ref == old(creation_num_ref) + 1;
    }
}
