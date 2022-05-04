script {
    use AptosFramework::ParallelExecutionConfig;
    fun main(aptos_root: signer, _execute_as: signer) {
        ParallelExecutionConfig::disable_parallel_execution(&aptos_root);
    }
}
