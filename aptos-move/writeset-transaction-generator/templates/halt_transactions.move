script {
    use DiemFramework::TransactionPublishingOption;
    fun main(diem_root: signer) {
        TransactionPublishingOption::halt_all_transactions(&diem_root);
    }
}
