module poc::is_permissioned_signer_impl {
    use velor_framework::permissioned_signer;

    public entry fun main(owner: &signer) {
        let _is_perm = permissioned_signer::is_permissioned_signer(owner);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
