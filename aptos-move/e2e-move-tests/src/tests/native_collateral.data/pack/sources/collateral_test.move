module collateral_test::collateral_test {
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_experimental::native_collateral;
    use aptos_experimental::native_collateral_types;
    use aptos_experimental::trading_native_capability::{Self, TradingNativeCapability};

    const SIGN_OFFSET: u64 = 9223372036854775808;

    /// The balance every `credit` adds to, shared by all senders.
    struct Exchange has key {
        cap: TradingNativeCapability,
        balance: Aggregator<u64>,
    }

    public entry fun register(framework: &signer, exchange: address) {
        trading_native_capability::register(framework, exchange);
    }

    public entry fun init(exchange: &signer) {
        move_to(exchange, Exchange {
            cap: trading_native_capability::get_capability(exchange),
            balance: aggregator_v2::create_unbounded_aggregator_with_value(SIGN_OFFSET),
        });
    }

    /// Adds `amount` to the shared balance and writes it as `account`'s cross
    /// collateral, without reading it.
    public entry fun credit(_sender: &signer, account: address, amount: u64) acquires Exchange {
        let exchange = &mut Exchange[@collateral_test];
        exchange.balance.add(amount);
        let collateral = native_collateral_types::new_collateral_v1(
            exchange.balance.snapshot(), vector[], 0
        );
        native_collateral::set_collateral(
            &exchange.cap, account, native_collateral_types::cross(), collateral
        );
    }

    /// Writes a fixed balance, for the abort tests.
    public entry fun set_balance(_sender: &signer, account: address, balance: i64) acquires Exchange {
        let exchange = &Exchange[@collateral_test];
        let collateral = native_collateral_types::new_collateral_v1_from_balance(
            balance, vector[], 0
        );
        native_collateral::set_collateral(
            &exchange.cap, account, native_collateral_types::cross(), collateral
        );
    }
}
