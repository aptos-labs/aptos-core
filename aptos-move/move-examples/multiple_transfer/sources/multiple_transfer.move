module transfer_account::multiple_transfer {
    use std::signer;
    use std::vector;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin;

    /// No addresses were provided
    const ENO_ADDRESSES: u64 = 1;

    /// Not enough funds to pay to all recipients
    const ENOT_ENOUGH_FUNDS: u64 = 2;

    /// Address vector length and amount vector length are not the same
    const ENO_ADDRESS_AMOUNT_MISMATCH: u64 = 3;

    /// Transfer to the list of addresses, using account transfer, and create the accounts
    /// if they don't exist.  This gives an example of how to handle errors, as well as
    /// handle iterating through a list of addresses.
    public entry fun transfer_same_amount(sender: &signer, addresses: vector<address>, amount: u64) {
        let num_addresses = vector::length(&addresses);
        assert!(num_addresses > 0, ENO_ADDRESSES);

        let sender_address = signer::address_of(sender);
        let total_balance = coin::balance<aptos_coin::AptosCoin>(sender_address);
        assert!(total_balance > (num_addresses * amount), ENOT_ENOUGH_FUNDS);

        let i = 0;
        while (i < num_addresses) {
            let receiver = vector::borrow(&addresses, i);
            aptos_account::transfer(sender, *receiver, amount);
            i = i + 1;
        };
    }

    /// Transfer to the list of addresses, using account transfer, and create the accounts
    /// if they don't exist.  This gives an example of how to handle errors, as well as
    /// handle iterating through a list of addresses and amounts
    public entry fun transfer_different_amounts(sender: &signer, addresses: vector<address>, amounts: vector<u64>) {
        let num_addresses = vector::length(&addresses);
        let num_amounts = vector::length(&amounts);
        assert!(num_addresses > 0, ENO_ADDRESSES);
        assert!(num_amounts == num_addresses, ENO_ADDRESS_AMOUNT_MISMATCH);

        let i = 0;
        while (i < num_addresses) {
            let receiver = vector::borrow(&addresses, i);
            let amount = vector::borrow(&amounts, i);
            aptos_account::transfer(sender, *receiver, *amount);
            i = i + 1;
        };
    }
}
