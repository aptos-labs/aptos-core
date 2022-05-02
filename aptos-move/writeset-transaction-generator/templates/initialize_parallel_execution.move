script {
    use AptosFramework::ParallelExecutionConfig;
    fun main(aptos_root: signer, _execute_as: signer) {
        ParallelExecutionConfig::initialize_parallel_execution(&aptos_root);
    }
}
