script {
    use aptos_framework::validator_set;
    fun main(aptos_root: signer) {
        {{#each addresses}}
        validator_set::remove_validator(&aptos_root, @0x{{this}});
        {{/each}}
    }
}
