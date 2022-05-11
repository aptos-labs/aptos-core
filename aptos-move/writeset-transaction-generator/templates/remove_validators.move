script {
    use AptosFramework::ValidatorSet;
    fun main(aptos_root: signer) {
        {{#each addresses}}
        ValidatorSet::remove_validator(&aptos_root, @0x{{this}});
        {{/each}}
    }
}
