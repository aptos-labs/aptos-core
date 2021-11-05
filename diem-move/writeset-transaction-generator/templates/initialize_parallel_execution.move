script {
    use DiemFramework::ParallelExecutionConfig;
    fun main(diem_root: signer, _execute_as: signer) {
        ParallelExecutionConfig::initialize_parallel_execution(&diem_root);
    }
}
