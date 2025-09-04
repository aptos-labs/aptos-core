script {
    use velor_framework::coin;
    use std::option::Option;
    use std::signer;


    fun main(
        first: &signer,
    ) {
        coin::balance<
            Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<
            Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<
            Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<
            Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<Option<u64>>>>>>>>>>>>>>
            >>>>>>>>>>>>>>
            >>>>>>>>>>>>>>
            >>>>>>>>>>>>>>
            >(signer::address_of(first));
    }
}
