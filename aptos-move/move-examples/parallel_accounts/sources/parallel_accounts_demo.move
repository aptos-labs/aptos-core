/// A demo of how multiple accounts can be used to delegate to a single resource
/// account for parallelism between transactions.
module parallel_accounts::parallel_accounts_demo {
    use aptos_framework::aptos_account;
    use aptos_token::token_transfers::offer_script;
    use std::string::String;
    use aptos_framework::account::{SignerCapability, create_resource_account, create_signer_with_capability, create_resource_address};
    use std::vector;
    use std::signer::address_of;

    /// No transfer admin capability found
    const ENO_ADMIN_CAP_FOUND: u64 = 1;
    /// No delegated transfer capability found
    const ENO_TRANSFER_CAP_FOUND: u64 = 2;
    /// No transfer resource found
    const ENO_TRANSFER_RESOURCE_FOUND: u64 = 3;

    const DEMO_SEED: vector<u8> = b"parallel_demo";

    /// Resource on the single transfer account
    struct TransferResource has key {
        signer_cap: SignerCapability,
    }

    /// Transfer admin resource, allows for delegation to accounts
    struct TransferAdmin has key {
        admin_cap: AdminCapability,
    }

    /// Transfer delegation account, which can only do transfers
    struct TransferDelegate has key, drop {
        transfer_cap: TransferCapability
    }

    /// Admin capability to delegate transfer actions
    struct AdminCapability has drop, store { account: address }

    /// Allows owner to do transfer actions
    struct TransferCapability has drop, store { account: address }

    /// Creates a resource account and stores the capability
    public entry fun create_resource_account_and_store_cap(sender: &signer, seed: vector<u8>) {
        let (resource_signer, signer_cap) = create_demo_resource_account(sender, seed);

        // Store
        move_to(&resource_signer, TransferResource {
            signer_cap
        });

        move_to(sender, TransferAdmin {
            admin_cap: AdminCapability {
                account: address_of(&resource_signer)
            },
        });
    }

    /// Adds a transfer cap to an account, transaction must be multi-agent
    public entry fun delegate_transfer_cap(admin: &signer, delegate: &signer) acquires TransferAdmin {
        let admin_account = address_of(admin);
        assert!(exists<TransferAdmin>(admin_account), ENO_ADMIN_CAP_FOUND);
        let admin = borrow_global<TransferAdmin>(admin_account);
        move_to(delegate, TransferDelegate {
            transfer_cap: TransferCapability {
                account: admin.admin_cap.account
            }
        });
    }

    /// Removes transfer cap from an account
    public entry fun revoke_transfer_cap(admin: &signer, delegate: address) acquires TransferDelegate {
        let admin_account = address_of(admin);
        assert!(exists<TransferAdmin>(admin_account), ENO_ADMIN_CAP_FOUND);
        assert!(exists<TransferDelegate>(delegate), ENO_TRANSFER_CAP_FOUND);
        move_from<TransferDelegate>(delegate);
    }

    /// Transfers coins with the TransferCapability
    public entry fun transfer_coins<CoinType>(
        payer: &signer,
        receiver: address,
        amount: u64
    ) acquires TransferResource, TransferDelegate {
        let resource_account = create_signer_for_delegate(payer);
        aptos_account::transfer_coins<CoinType>(&resource_account, receiver, amount)
    }

    public entry fun offer_token<CoinType>(
        payer: &signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64
    ) acquires TransferResource, TransferDelegate {
        let resource_account = create_signer_for_delegate(payer);
        offer_script(resource_account, receiver, creator, collection, name, property_version, amount)
    }

    #[view]
    /// Retrieves the resource account address used with this contract
    public fun resource_account_address(payer: address, seed: vector<u8>): address {
        let seed = create_seed(seed);
        create_resource_address(&payer, seed)
    }

    inline fun create_seed(seed: vector<u8>): vector<u8> {
        vector::append(&mut seed, DEMO_SEED);
        seed
    }

    inline fun create_demo_resource_account(payer: &signer, seed: vector<u8>): (signer, SignerCapability) {
        let seed = create_seed(seed);
        create_resource_account(payer, seed)
    }

    inline fun create_signer_for_delegate(payer: &signer): signer {
        // Retrieve the transfer delegation
        let payer_address = address_of(payer);
        assert!(exists<TransferDelegate>(payer_address), ENO_TRANSFER_CAP_FOUND);
        let delegate = borrow_global<TransferDelegate>(payer_address);

        // Build a signer form the account
        let resource_account = delegate.transfer_cap.account;
        assert!(exists<TransferResource>(resource_account), ENO_TRANSFER_RESOURCE_FOUND);
        let transfer_resource = borrow_global<TransferResource>(resource_account);
        create_signer_with_capability(&transfer_resource.signer_cap)
    }
}
