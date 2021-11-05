script {
    use DiemFramework::ParallelExecutionConfig;
    fun main(diem_root: signer, _execute_as: signer, payload: vector<u8>) {
        ParallelExecutionConfig::enable_parallel_execution_with_config(&diem_root, payload);
    }
}
