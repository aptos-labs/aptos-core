#[contract]
/// Faucet example in the Ethereum book.
module 0x42::Faucet {
    use Evm::Evm::{sender, value, self, sign, balance, transfer, emit};
    use Std::Errors;

    #[storage]
    struct State has key {
        owner: address,
    }

    #[event]
    struct WithdrawalEvent {
        to: address,
        amount: u128
    }

    #[event]
    struct DepositEvent {
        from: address,
        amount: u128
    }

    #[create]
    public fun create() {
        move_to<State>(sign(self()), State{owner: sender()})
    }

    #[delete]
    public fun delete() acquires State {
        let state = borrow_global<State>(self());
        assert!(sender() == state.owner, Errors::requires_address(0));
    }

    #[receive, payable]
    public fun receive() {
        emit(DepositEvent{from: sender(), amount: value()})
    }

    #[callable]
    public fun withdraw(amount: u128) acquires State {
        let state = borrow_global<State>(self());

        // Don't allow to withdraw from self.
        assert!(state.owner != self(), Errors::invalid_argument(0));

        // Limit withdrawal amount
        assert!(amount <= 100, Errors::invalid_argument(0));

        // Funds must be available.
        assert!(balance(self()) >= amount, Errors::limit_exceeded(0));

        // Transfer funds
        transfer(sender(), amount);
        emit(WithdrawalEvent{to: sender(), amount})
    }
}
