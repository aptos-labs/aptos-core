script {
    use aptos_framework::transaction_publishing_option;
    fun main(aptos_root: signer) {
        transaction_publishing_option::halt_all_transactions(&aptos_root);
    }
}
