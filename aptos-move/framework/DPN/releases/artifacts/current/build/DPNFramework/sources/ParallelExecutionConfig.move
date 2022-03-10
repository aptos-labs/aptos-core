/// This module defines structs and methods to initialize VM configurations,
/// including different costs of running the VM.
module DiemFramework::ParallelExecutionConfig {
    use DiemFramework::Reconfiguration::{Self, Reconfiguration};
    use DiemFramework::Timestamp;
    use DiemFramework::Roles;
    use Std::Option::{Self, Option};

    /// The struct to hold the read/write set analysis result for the whole Diem Framework.
    struct ParallelExecutionConfig has copy, drop, store {
        /// Serialized analysis result for the Diem Framework.
        /// If this payload is not None, DiemVM will use this config to execute transactions in parallel.
        read_write_analysis_result: Option<vector<u8>>
    }

    /// Enable parallel execution functionality of DiemVM by setting the read_write_set analysis result.
    public fun initialize_parallel_execution(
        dr_account: &signer,
    ) {
        // The permission "UpdateVMConfig" is granted to DiemRoot [[H11]][PERMISSION].
        Roles::assert_diem_root(dr_account);
        Reconfiguration::publish_new_config(
            dr_account,
            ParallelExecutionConfig {
                read_write_analysis_result: Option::none(),
            },
        );
    }

    public fun enable_parallel_execution_with_config(
       dr_account: &signer,
       read_write_inference_result: vector<u8>,
    ) {
        Timestamp::assert_operating();
        Roles::assert_diem_root(dr_account);
        Reconfiguration::set(dr_account, ParallelExecutionConfig {
            read_write_analysis_result: Option::some(read_write_inference_result),
        });
    }

    public fun disable_parallel_execution(
       dr_account: &signer,
    ) {
        Timestamp::assert_operating();
        Roles::assert_diem_root(dr_account);
        Reconfiguration::set(dr_account, ParallelExecutionConfig {
            read_write_analysis_result: Option::none(),
        });
    }

    spec initialize_parallel_execution {
        /// Must abort if the signer does not have the DiemRoot role [[H11]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};

        include Reconfiguration::PublishNewConfigAbortsIf<ParallelExecutionConfig>;
        include Reconfiguration::PublishNewConfigEnsures<ParallelExecutionConfig> {
            payload: ParallelExecutionConfig {
                read_write_analysis_result: Option::none(),
            }};
    }

    spec enable_parallel_execution_with_config {
        include Timestamp::AbortsIfNotOperating;
        /// No one can update VMConfig except for the Diem Root account [[H11]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};
        include Reconfiguration::SetAbortsIf<ParallelExecutionConfig>{account: dr_account };
        ensures Reconfiguration::spec_is_published<ParallelExecutionConfig>();

        // TODO: How to replace this assertion since we can't invoke Option::some here?
        //        ensures Reconfiguration::get<ParallelExecutionConfig>() == ParallelExecutionConfig {
        //            read_write_analysis_result: result,
        //        };

        ensures old(Reconfiguration::spec_has_config()) == Reconfiguration::spec_has_config();
    }

    spec disable_parallel_execution {
        include Timestamp::AbortsIfNotOperating;
        /// No one can update VMConfig except for the Diem Root account [[H11]][PERMISSION].
        include Roles::AbortsIfNotDiemRoot{account: dr_account};
        include Reconfiguration::SetAbortsIf<ParallelExecutionConfig>{account: dr_account };
        ensures Reconfiguration::spec_is_published<ParallelExecutionConfig>();
        ensures Reconfiguration::get<ParallelExecutionConfig>() == ParallelExecutionConfig {
            read_write_analysis_result: Option::none(),
        };
        ensures old(Reconfiguration::spec_has_config()) == Reconfiguration::spec_has_config();
    }


    spec module { } // Switch documentation context to module level.

    /// # Access Control

    /// The permission "UpdateParallelExecutionConfig" is granted to DiemRoot [[H11]][PERMISSION].
    spec module {
        invariant [suspendable] forall addr: address
            where exists<Reconfiguration<ParallelExecutionConfig>>(addr): addr == @DiemRoot;

        invariant update [suspendable] old(Reconfiguration::spec_is_published<ParallelExecutionConfig>())
            && Reconfiguration::spec_is_published<ParallelExecutionConfig>()
            && old(Reconfiguration::get<ParallelExecutionConfig>()) != Reconfiguration::get<ParallelExecutionConfig>()
                ==> Roles::spec_signed_by_diem_root_role();
    }

    // TODO: The following is the old style spec, which can removed later.
    /// No one can update VMConfig except for the Diem Root account [[H11]][PERMISSION].
    spec schema VMConfigRemainsSame {
        ensures old(Reconfiguration::spec_is_published<ParallelExecutionConfig>()) ==>
            global<Reconfiguration<ParallelExecutionConfig>>(@DiemRoot) ==
                old(global<Reconfiguration<ParallelExecutionConfig>>(@DiemRoot));
    }
    spec module {
        apply VMConfigRemainsSame to * except enable_parallel_execution_with_config, disable_parallel_execution;
    }
}
