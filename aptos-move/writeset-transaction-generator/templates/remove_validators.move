script {
    use DiemFramework::ValidatorSystem;
    fun main(diem_root: signer) {
        {{#each addresses}}
        ValidatorSystem::remove_validator(&diem_root, @0x{{this}});
        {{/each}}
    }
}
