/// Basic faucet, allows to request coins between intervals.
module SwapDeployer::FaucetV1 {
    use std::signer;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin};

    // Errors.

    /// When Faucet already exists on account.
    const ERR_FAUCET_EXISTS: u64 = 100;

    /// When Faucet doesn't exists on account.
    const ERR_FAUCET_NOT_EXISTS: u64 = 101;

    /// When user already got coins and currently restricted to request more funds.
    const ERR_RESTRICTED: u64 = 102;

    /// Faucet data.
    struct Faucet<phantom CoinType> has key {
        /// Faucet balance.
        deposit: Coin<CoinType>,
        /// How much coins should be sent to user per request.
        per_request: u64,
        /// Period between requests to faucet in seconds.
        period: u64,
    }

    /// If user has this resource on his account - he's not able to get more funds if (current_timestamp < since + period).
    struct Restricted<phantom Faucet> has key {
        since: u64,
    }

    // Public functions.

    /// Create a new faucet on `account` address.
    /// * `deposit` - initial coins on faucet balance.
    /// * `per_request` - how much funds should be distributed per user request.
    /// * `period` - interval allowed between requests for specific user.
    public fun create_faucet_internal<CoinType>(account: &signer, deposit: Coin<CoinType>, per_request: u64, period: u64) {
        let account_addr = signer::address_of(account);

        assert!(!exists<Faucet<CoinType>>(account_addr), ERR_FAUCET_EXISTS);

        move_to(account, Faucet<CoinType> {
            deposit,
            per_request,
            period
        });
    }

    /// Change settings of faucet `CoinType`.
    /// * `per_request` - how much funds should be distributed per user request.
    /// * `period` - interval allowed between requests for specific user.
    public fun change_settings_internal<CoinType>(account: &signer, per_request: u64, period: u64) acquires Faucet {
        let account_addr = signer::address_of(account);

        assert!(exists<Faucet<CoinType>>(account_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(account_addr);
        faucet.per_request = per_request;
        faucet.period = period;
    }

    /// Deposist more coins `CoinType` to faucet.
    public fun deposit_internal<CoinType>(faucet_addr: address, deposit: Coin<CoinType>) acquires Faucet {
        assert!(exists<Faucet<CoinType>>(faucet_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(faucet_addr);
        coin::merge(&mut faucet.deposit, deposit);
    }

    /// Requests coins `CoinType` from faucet `faucet_addr`.
    public fun request_internal<CoinType>(account: &signer, faucet_addr: address): Coin<CoinType> acquires Faucet, Restricted {
        let account_addr = signer::address_of(account);

        assert!(exists<Faucet<CoinType>>(faucet_addr), ERR_FAUCET_NOT_EXISTS);

        let faucet = borrow_global_mut<Faucet<CoinType>>(faucet_addr);
        let coins = coin::extract(&mut faucet.deposit, faucet.per_request);

        let now = timestamp::now_seconds();

        if (exists<Restricted<CoinType>>(account_addr)) {
            let restricted = borrow_global_mut<Restricted<CoinType>>(account_addr);
            assert!(restricted.since + faucet.period <= now, ERR_RESTRICTED);
            restricted.since = now;
        } else {
            move_to(account, Restricted<CoinType> {
                since: now,
            });
        };

        coins
    }

    // Scripts.

    /// Creates new faucet on `account` address for coin `CoinType`.
    /// * `account` - account which creates
    /// * `per_request` - how much funds should be distributed per user request.
    /// * `period` - interval allowed between requests for specific user.
    public entry fun create_faucet<CoinType>(account: &signer, amount_to_deposit: u64, per_request: u64, period: u64) {
        let coins = coin::withdraw<CoinType>(account, amount_to_deposit);

        create_faucet_internal(account, coins, per_request, period);
    }

    /// Changes faucet settings on `account`.
    public entry fun change_settings<CoinType>(account: &signer, per_request: u64, period: u64) acquires Faucet {
        change_settings_internal<CoinType>(account, per_request, period);
    }

    /// Deposits coins `CoinType` to faucet on `faucet` address, withdrawing funds from user balance.
    public entry fun deposit<CoinType>(account: &signer, faucet_addr: address, amount: u64) acquires Faucet {
        let coins = coin::withdraw<CoinType>(account, amount);

        deposit_internal<CoinType>(faucet_addr, coins);
    }

    /// Deposits coins `CoinType` from faucet on user's account.
    /// `faucet` - address of faucet to request funds.
    public entry fun request<CoinType>(account: &signer, faucet_addr: address) acquires Faucet, Restricted {
        let account_addr = signer::address_of(account);

        if (!coin::is_account_registered<CoinType>(account_addr)) {
            coin::register<CoinType>(account);
        };

        let coins = request_internal<CoinType>(account, faucet_addr);

        coin::deposit(account_addr, coins);
    }

    #[test_only]
    use aptos_framework::genesis;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_framework::account::create_account;

    #[test_only]
    struct FakeMoney has store {}

    #[test_only]
    struct FakeMoneyCaps has key {
        mint_cap: coin::MintCapability<FakeMoney>,
        burn_cap: coin::BurnCapability<FakeMoney>,
    }

    #[test(core = @core_resources, faucet_creator = @SwapDeployer, someone_else = @0x11)]
    public entry fun test_faucet_end_to_end(core: &signer, faucet_creator: &signer, someone_else: &signer) acquires Faucet, Restricted {
        genesis::setup(core);

        create_account(signer::address_of(faucet_creator));
        create_account(signer::address_of(someone_else));

        let (m, b) = coin::initialize<FakeMoney>(
            faucet_creator,
            utf8(b"FakeMoney"),
            utf8(b"FM"),
            8,
            true
        );

        let amount = 100000000000000u64;
        let per_request = 1000000000u64;
        let period = 3000u64;

        let faucet_addr = signer::address_of(faucet_creator);

        let coins_minted = coin::mint(amount, &m);
        coin::register<FakeMoney>(faucet_creator);
        coin::deposit(faucet_addr, coins_minted);

        create_faucet<FakeMoney>(faucet_creator, amount / 2, per_request, period);

        request<FakeMoney>(faucet_creator, faucet_addr);
        assert!(coin::balance<FakeMoney>(faucet_addr) == (amount / 2 + per_request), 0);

        let someone_else_addr = signer::address_of(someone_else);
        request<FakeMoney>(someone_else, faucet_addr);
        assert!(coin::balance<FakeMoney>(someone_else_addr) == per_request, 1);

        timestamp::update_global_time_for_test(3000000000);

        let new_per_request = 2000000000u64;
        change_settings<FakeMoney>(faucet_creator, new_per_request, period);

        request<FakeMoney>(someone_else, faucet_addr);
        assert!(coin::balance<FakeMoney>(someone_else_addr) == (per_request + new_per_request), 2);

        change_settings<FakeMoney>(faucet_creator, new_per_request, 5000);
        let to_check = borrow_global<Faucet<FakeMoney>>(faucet_addr);
        assert!(to_check.period == 5000, 3);
        assert!(to_check.per_request == new_per_request, 4);

        deposit<FakeMoney>(someone_else, faucet_addr, new_per_request);
        assert!(coin::balance<FakeMoney>(someone_else_addr) == per_request, 5);

        move_to(faucet_creator, FakeMoneyCaps {
            mint_cap: m,
            burn_cap: b,
        });
    }

    #[test(core = @core_resources, faucet_creator = @SwapDeployer, someone_else = @0x11)]
    #[expected_failure(abort_code = 102)]
    public entry fun test_faucet_fail_request(core: &signer, faucet_creator: &signer, someone_else: &signer) acquires Faucet, Restricted {
        genesis::setup(core);

        create_account(signer::address_of(faucet_creator));
        create_account(signer::address_of(someone_else));

        let (m, b) = coin::initialize<FakeMoney>(
            faucet_creator,
            utf8(b"FakeMoney"),
            utf8(b"FM"),
            8,
            true
        );

        let amount = 100000000000000u64;
        let per_request = 1000000000u64;
        let period = 3000u64;

        let faucet_addr = signer::address_of(faucet_creator);

        let coins_minted = coin::mint(amount, &m);
        coin::register<FakeMoney>(faucet_creator);
        coin::deposit(faucet_addr, coins_minted);

        create_faucet<FakeMoney>(faucet_creator, amount / 2, per_request, period);

        request<FakeMoney>(faucet_creator, faucet_addr);
        request<FakeMoney>(faucet_creator, faucet_addr);
        assert!(coin::balance<FakeMoney>(faucet_addr) == (amount / 2 + per_request), 0);

        move_to(faucet_creator, FakeMoneyCaps{
            mint_cap: m,
            burn_cap: b,
        });
    }

    #[test(core = @core_resources, faucet_creator = @SwapDeployer, someone_else = @0x11)]
    #[expected_failure(abort_code = 101)]
    public entry fun test_faucet_fail_settings(core: &signer, faucet_creator: &signer, someone_else: &signer) acquires Faucet {
        genesis::setup(core);

        create_account(signer::address_of(faucet_creator));
        create_account(signer::address_of(someone_else));

        let (m, b) = coin::initialize<FakeMoney>(
            faucet_creator,
            utf8(b"FakeMoney"),
            utf8(b"FM"),
            8,
            true
        );

        let amount = 100000000000000u64;
        let per_request = 1000000000u64;
        let period = 3000u64;

        let faucet_addr = signer::address_of(faucet_creator);

        let coins_minted = coin::mint(amount, &m);
        coin::register<FakeMoney>(faucet_creator);
        coin::deposit(faucet_addr, coins_minted);

        create_faucet<FakeMoney>(faucet_creator, amount / 2, per_request, period);
        change_settings<FakeMoney>(someone_else, 1, 1);

        move_to(faucet_creator, FakeMoneyCaps{
            mint_cap: m,
            burn_cap: b,
        });
    }

    #[test(core = @core_resources, faucet_creator = @SwapDeployer, someone_else = @0x11)]
    #[expected_failure(abort_code = 100)]
    public entry fun test_already_exists(core: &signer, faucet_creator: &signer, someone_else: &signer) {
        genesis::setup(core);

        create_account(signer::address_of(faucet_creator));
        create_account(signer::address_of(someone_else));

        let (m, b) = coin::initialize<FakeMoney>(
            faucet_creator,
            utf8(b"FakeMoney"),
            utf8(b"FM"),
            8,
            true
        );

        let amount = 100000000000000u64;
        let per_request = 1000000000u64;
        let period = 3000u64;

        let faucet_addr = signer::address_of(faucet_creator);

        let coins_minted = coin::mint(amount, &m);
        coin::register<FakeMoney>(faucet_creator);
        coin::deposit(faucet_addr, coins_minted);

        create_faucet<FakeMoney>(faucet_creator, amount / 2, per_request, period);
        create_faucet<FakeMoney>(faucet_creator, amount / 2, per_request, period);

        move_to(faucet_creator, FakeMoneyCaps{
            mint_cap: m,
            burn_cap: b,
        });
    }
}