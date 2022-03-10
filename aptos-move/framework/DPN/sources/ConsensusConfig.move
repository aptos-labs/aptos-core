/// Maintains the consensus config for the Diem blockchain. The config is stored in a
/// Reconfiguration, and may be updated by Diem root.
module DiemFramework::ConsensusConfig {
    use DiemFramework::Reconfiguration::{Self, Reconfiguration};
    use DiemFramework::Roles;
    use Std::Vector;

    struct ConsensusConfig has copy, drop, store {
        config: vector<u8>,
    }

    /// Publishes the ConsensusConfig config.
    public fun initialize(dr_account: &signer) {
        Roles::assert_diem_root(dr_account);
        Reconfiguration::publish_new_config(dr_account, ConsensusConfig { config: Vector::empty() });
    }
    spec initialize {
        /// Must abort if the signer does not have the DiemRoot role [[H12]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};

        include Reconfiguration::PublishNewConfigAbortsIf<ConsensusConfig>;
        include Reconfiguration::PublishNewConfigEnsures<ConsensusConfig>{
            payload: ConsensusConfig { config: Vector::empty() }
        };
    }

    /// Allows Diem root to update the config.
    public fun set(dr_account: &signer, config: vector<u8>) {
        Roles::assert_diem_root(dr_account);

        Reconfiguration::set(
            dr_account,
            ConsensusConfig { config }
        );
    }
    spec set {
        /// Must abort if the signer does not have the DiemRoot role [[H12]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};

        include Reconfiguration::SetAbortsIf<ConsensusConfig>{account: dr_account};
        include Reconfiguration::SetEnsures<ConsensusConfig>{payload: ConsensusConfig { config }};
    }

    // =================================================================
    // Module Specification

    spec module {} // Switch to module documentation context

    /// # Access Control

    /// The permission "UpdateConsensusConfig" is granted to DiemRoot [[H12]][PERMISSION].
    spec module {
        invariant [suspendable] forall addr: address
            where exists<Reconfiguration<ConsensusConfig>>(addr): addr == @DiemRoot;

        invariant update [suspendable] old(Reconfiguration::spec_is_published<ConsensusConfig>())
            && Reconfiguration::spec_is_published<ConsensusConfig>()
            && old(Reconfiguration::get<ConsensusConfig>()) != Reconfiguration::get<ConsensusConfig>()
                ==> Roles::spec_signed_by_diem_root_role();
    }

    // TODO: The following is the old style spec, which can removed later.
    /// Only "set" can modify the ConsensusConfig config [[H12]][PERMISSION]
    spec schema ConsensusConfigRemainsSame {
        ensures old(Reconfiguration::spec_is_published<ConsensusConfig>()) ==>
            global<Reconfiguration<ConsensusConfig>>(@DiemRoot) ==
                old(global<Reconfiguration<ConsensusConfig>>(@DiemRoot));
    }
    spec module {
        apply ConsensusConfigRemainsSame to * except set;
    }
}
