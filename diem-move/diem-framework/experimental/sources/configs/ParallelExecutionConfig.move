/// This module defines structs and methods to initialize VM configurations,
/// including different costs of running the VM.
module ExperimentalFramework::ParallelExecutionConfig {
    use ExperimentalFramework::DiemConfig;
    use CoreFramework::DiemTimestamp;
    use CoreFramework::SystemAddresses;
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
        SystemAddresses::assert_core_resource(dr_account);
        DiemConfig::publish_new_config(
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
        DiemTimestamp::assert_operating();
        SystemAddresses::assert_core_resource(dr_account);
        DiemConfig::set(dr_account, ParallelExecutionConfig {
            read_write_analysis_result: Option::some(read_write_inference_result),
        });
    }

    public fun disable_parallel_execution(
       dr_account: &signer,
    ) {
        DiemTimestamp::assert_operating();
        SystemAddresses::assert_core_resource(dr_account);
        DiemConfig::set(dr_account, ParallelExecutionConfig {
            read_write_analysis_result: Option::none(),
        });
    }
}
