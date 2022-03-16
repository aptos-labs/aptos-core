/// This module defines structs and methods to initialize VM configurations,
/// including different costs of running the VM.
module CoreFramework::ParallelExecutionConfig {
    use Std::Capability::Cap;
    use Std::Errors;
    use Std::Option::{Self, Option};
    use CoreFramework::Reconfiguration;
    use CoreFramework::Timestamp;
    use CoreFramework::SystemAddresses;

    /// Error with chain marker
    const ECHAIN_MARKER: u64 = 0;
    /// Error with config
    const ECONFIG: u64 = 1;

    /// Marker to be stored under @CoreResources during genesis
    struct ParallelExecutionConfigChainMarker<phantom T> has key {}

    /// The struct to hold the read/write set analysis result for the whole core Framework.
    struct ParallelExecutionConfig has key {
        /// Serialized analysis result for the core Framework.
        /// If this payload is not None, VM will use this config to execute transactions in parallel.
        read_write_analysis_result: Option<vector<u8>>
    }

    /// Enable parallel execution functionality of VM by setting the read_write_set analysis result.
    public fun initialize_parallel_execution<T>(
        account: &signer,
    ) {
        Timestamp::assert_genesis();
        SystemAddresses::assert_core_resource(account);

        assert!(
            !exists<ParallelExecutionConfigChainMarker<T>>(@CoreResources),
            Errors::already_published(ECHAIN_MARKER)
        );

        assert!(
            !exists<ParallelExecutionConfig>(@CoreResources),
            Errors::already_published(ECONFIG)
        );

        move_to(account, ParallelExecutionConfigChainMarker<T>{});

        move_to(
            account,
            ParallelExecutionConfig {
                read_write_analysis_result: Option::none(),
            },
        );
    }

    public fun enable_parallel_execution_with_config<T>(
        read_write_inference_result: vector<u8>,
        _cap: &Cap<T>
    ) acquires ParallelExecutionConfig {
        Timestamp::assert_operating();
        assert!(
            exists<ParallelExecutionConfigChainMarker<T>>(@CoreResources),
            Errors::not_published(ECHAIN_MARKER)
        );
        let result_ref = &mut borrow_global_mut<ParallelExecutionConfig>(@CoreResources).read_write_analysis_result;
        *result_ref = Option::some(read_write_inference_result);
        Reconfiguration::reconfigure();
    }

    public fun disable_parallel_execution<T>(
        _cap: &Cap<T>
    ) acquires ParallelExecutionConfig {
        Timestamp::assert_operating();
        assert!(
            exists<ParallelExecutionConfigChainMarker<T>>(@CoreResources),
            Errors::not_published(ECHAIN_MARKER)
        );
        let result_ref = &mut borrow_global_mut<ParallelExecutionConfig>(@CoreResources).read_write_analysis_result;
        *result_ref = Option::none();
        Reconfiguration::reconfigure();
    }
}
