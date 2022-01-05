module ExperimentalFramework::ExperimentalParallelExecutionConfig {
    use Std::Capability;
    use CoreFramework::ParallelExecutionConfig;

    struct ExperimentalParallelExecutionConfig has drop {}

    public fun initialize_parallel_execution(
        account: &signer,
    ) {
        ParallelExecutionConfig::initialize_parallel_execution<ExperimentalParallelExecutionConfig>(account);
        Capability::create<ExperimentalParallelExecutionConfig>(
            account,
            &ExperimentalParallelExecutionConfig {}
        );
    }

    public fun enable_parallel_execution_with_config(
        account: &signer,
        read_write_inference_result: vector<u8>,
    ) {
        ParallelExecutionConfig::enable_parallel_execution_with_config(
            read_write_inference_result,
            &Capability::acquire(account, &ExperimentalParallelExecutionConfig {}),
        );
    }

    public fun disable_parallel_execution(account: &signer) {
        ParallelExecutionConfig::disable_parallel_execution(
            &Capability::acquire(account, &ExperimentalParallelExecutionConfig {}),
        );
    }
}
