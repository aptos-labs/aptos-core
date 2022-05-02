script {
    use AptosFramework::TransactionPublishingOption;
    fun main(aptos_root: signer) {
        TransactionPublishingOption::halt_all_transactions(&aptos_root);
    }
}
