/// Maintains the version number for the Diem blockchain. The version is stored in a
/// Reconfiguration, and may be updated by Diem root.
module DiemFramework::Version {
    use DiemFramework::Reconfiguration::{Self, Reconfiguration};
    use DiemFramework::Timestamp;
    use DiemFramework::Roles;
    use Std::Errors;

    struct Version has copy, drop, store {
        major: u64,
    }

    /// Tried to set an invalid major version for the VM. Major versions must be strictly increasing
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 0;

    /// Publishes the Version config. Must be called during Genesis.
    public fun initialize(dr_account: &signer, initial_version: u64) {
        Timestamp::assert_genesis();
        Roles::assert_diem_root(dr_account);
        Reconfiguration::publish_new_config<Version>(
            dr_account,
            Version { major: initial_version },
        );
    }
    spec initialize {
        /// Must abort if the signer does not have the DiemRoot role [[H10]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};

        include Timestamp::AbortsIfNotGenesis;
        include Reconfiguration::PublishNewConfigAbortsIf<Version>;
        include Reconfiguration::PublishNewConfigEnsures<Version>{payload: Version { major: initial_version }};
    }

    /// Allows Diem root to update the major version to a larger version.
    public fun set(dr_account: &signer, major: u64) {
        Timestamp::assert_operating();

        Roles::assert_diem_root(dr_account);

        let old_config = Reconfiguration::get<Version>();

        assert!(
            old_config.major < major,
            Errors::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER)
        );

        Reconfiguration::set<Version>(
            dr_account,
            Version { major }
        );
    }
    spec set {
        /// Must abort if the signer does not have the DiemRoot role [[H10]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};

        include Timestamp::AbortsIfNotOperating;
        aborts_if Reconfiguration::get<Version>().major >= major with Errors::INVALID_ARGUMENT;
        include Reconfiguration::SetAbortsIf<Version>{account: dr_account};
        include Reconfiguration::SetEnsures<Version>{payload: Version { major }};
    }

    // =================================================================
    // Module Specification

    spec module {} // Switch to module documentation context

    /// # Initialization
    spec module {
        /// After genesis, version is published.
        invariant [suspendable] Timestamp::is_operating() ==> Reconfiguration::spec_is_published<Version>();
    }

    /// # Access Control

    /// The permission "UpdateDiemProtocolVersion" is granted to DiemRoot [[H10]][PERMISSION].
    spec module {
        invariant [suspendable] forall addr: address
            where exists<Reconfiguration<Version>>(addr): addr == @DiemRoot;

        invariant update [suspendable] old(Reconfiguration::spec_is_published<Version>())
            && Reconfiguration::spec_is_published<Version>()
            && old(Reconfiguration::get<Version>().major) != Reconfiguration::get<Version>().major
                ==> Roles::spec_signed_by_diem_root_role();
    }

    // TODO: The following is the old style spec, which can removed later.
    /// Only "set" can modify the Version config [[H10]][PERMISSION]
    spec schema VersionRemainsSame {
        ensures old(Reconfiguration::spec_is_published<Version>()) ==>
            global<Reconfiguration<Version>>(@DiemRoot) ==
                old(global<Reconfiguration<Version>>(@DiemRoot));
    }
    /// The permission "UpdateDiemProtocolVersion" is granted to DiemRoot [[H10]][PERMISSION].
    spec module {
        apply VersionRemainsSame to * except set;
    }

    /// # Other Invariants
    spec module {
        /// Version number never decreases
        invariant update [suspendable]
            Timestamp::is_operating() ==>
                (old(Reconfiguration::get<Version>().major) <= Reconfiguration::get<Version>().major);
    }

}
