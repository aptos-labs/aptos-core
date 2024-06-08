module 0x1::test {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::randomness;
    use aptos_framework::coin;

    entry fun ok_if_not_annotated_and_not_using_randomness() {
        // Do nothing.
    }

    #[randomness]
    entry fun ok_if_annotated_and_not_using_randomness() {
        // Do nothing.
    }

    #[randomness]
    entry fun ok_if_annotated_and_using_randomness() {
        let _ = randomness::u64_integer();
    }

    #[lint::allow_unsafe_randomness]
    public entry fun fail_if_not_annotated_and_using_randomness() {
        let _ = randomness::u64_integer();
    }

    #[randomness]
    /// Transfer some amount out to 2 recipients with a random split.
    entry fun transfer_lucky_money(sender: &signer, amount: u64, recipient_0: address, recipient_1: address) {
        let part_0 = randomness::u64_range(0, amount + 1);
        let part_1 = amount - part_0;
        coin::transfer<AptosCoin>(sender, recipient_0, part_0);
        coin::transfer<AptosCoin>(sender, recipient_1, part_1);
    }
}
