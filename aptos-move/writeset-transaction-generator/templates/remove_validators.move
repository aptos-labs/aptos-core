script {
    use DiemFramework::ValidatorSet;
    fun main(diem_root: signer) {
        {{#each addresses}}
        ValidatorSet::remove_validator(&diem_root, @0x{{this}});
        {{/each}}
    }
}
