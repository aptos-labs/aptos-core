script {
    use aptos_framework::parallel_execution_config;
    fun main(aptos_root: signer, _execute_as: signer) {
        parallel_execution_config::initialize_parallel_execution(&aptos_root);
    }
}
