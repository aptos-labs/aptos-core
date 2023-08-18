module aptos_framework::scheduled_transaction {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::event::{EventHandle, emit_event};
    use aptos_framework::timestamp;
    use aptos_std::smart_vector::{Self, SmartVector};
    use std::error;
    use std::signer;
    use std::string::String;
    use std::vector;

    struct ScheduledTransactions has key {
        transactions: SmartVector<Object<ScheduledTransaction>>,
    }

    struct TransactionToExecute {
        sender: address,
        transaction: address,
        payload: vector<u8>,
        max_gas_price: u64,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct ScheduledTransaction has key {
        sender: address,
        payload: vector<u8>,
        gas: Coin<AptosCoin>,
        max_gas_price: u64,
        recurrences: u64,
        period: u64,
        last_execution_time_secs: u64,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct ScheduledTransactionEvents has copy, key {
        create_transaction_events: EventHandle<CreateTransactionEvent>,
        transaction_execution_succeeded_events: EventHandle<TransactionExecutionSucceededEvent>,
        transaction_execution_failed_events: EventHandle<TransactionExecutionFailedEvent>,
    }

    struct CreateTransactionEvent has drop, store {
        sender: address,
        payload: vector<u8>,
        gas: u64,
        max_gas_price: u64,
        retries: u64,
    }

    struct TransactionExecutionSucceededEvent has drop, store {
        sender: address,
        payload: vector<u8>,
    }

    struct ExecutionError has copy, drop, store {
        // The module where the error occurs.
        abort_location: String,
        // There are 3 error types, stored as strings:
        // 1. VMError. Indicates an error from the VM, e.g. out of gas, invalid auth key, etc.
        // 2. MoveAbort. Indicates an abort, e.g. assertion failure, from inside the executed Move code.
        // 3. MoveExecutionFailure. Indicates an error from Move code where the VM could not continue. For example,
        // arithmetic failures.
        error_type: String,
        // The detailed error code explaining which error occurred.
        error_code: u64,
    }

    struct TransactionExecutionFailedEvent has drop, store {
        sender: address,
        payload: u64,
        execution_error: ExecutionError,
    }

    public fun initialize(aptos_framework: &signer) {
        move_to(aptos_framework, ScheduledTransactions {
            transactions: smart_vector::new(),
        });
    }

    public entry fun schedule_transaction(
        sender: &signer,
        payload: vector<u8>,
        gas: u64,
        max_gas_price: u64,
        recurrences: u64,
        period: u64,
        last_execution_time_secs: u64,
    ) {
        let constructor_ref = &object::create_object(creator_address);
        let scheduled_tx_signer = object::generate_signer(constructor_ref);
        move_to(scheduled_tx_signer, ScheduledTransaction {
            sender: signer::address_of(sender),
            payload,
            gas: coin::withdraw<AptosCoin>(sender, gas),
            max_gas_price,
            recurrences,
            period,
            last_execution_time_secs,
        });
    }

    ////////////////////////// To be called by VM only ///////////////////////////////

    fun get_transactions_to_execute(limit: u64): vector<TransactionToExecute> acquires ScheduledTransactions {
        let scheduled_transactions = &mut borrow_global_mut<ScheduledTransactions>(@aptos_framework).transactions;
        let executable_transactions = vector[];
        let now = timestamp::now_seconds();
        while (smart_vector::len(scheduled_transactions) > 0) {
            let transaction = smart_vector::borrow(scheduled_transactions, 0);
            let transaction_addr = object::object_address(transaction);
            let transaction_data = borrow_global_mut<ScheduledTransaction>(transaction_addr);
            if (transaction_data.last_execution_time_secs + transaction_data.period <= now) {
                vector::push_back(executable_transactions, TransactionToExecute {
                    sender: transaction_data.sender,
                    transaction: transaction_addr,
                    payload: transaction_data.payload,
                    max_gas_price: transaction_data.max_gas_price,
                });
                transaction.last_execution_time_secs = now;
                transaction.recurrences = transaction.recurrences - 1;
                if (transaction.recurrences > 0) {
                    smart_vector::push_back(scheduled_transactions, transaction);
                };
            } else {
                break;
            };
        };
    }

    fun successful_transaction_execution_cleanup(transaction: address) acquires ScheduledTransaction {
        let scheduled_transaction = borrow_global<ScheduledTransaction>(transaction);
        let scheduled_transaction_events = borrow_global_mut<ScheduledTransactionEvents>(transaction);
        emit_event(
            &mut scheduled_transaction_events.transaction_execution_succeeded_events,
            TransactionExecutionSucceededEvent {
                sender: scheduled_transaction.sender,
                payload: scheduled_transaction.payload,
            }
        );
    }

    fun failed_transaction_execution_cleanup(
        transaction: address,
        execution_error: ExecutionError,
    ) acquires ScheduledTransaction {
        let scheduled_transaction = borrow_global<ScheduledTransaction>(transaction);
        let scheduled_transaction_events = borrow_global_mut<ScheduledTransactionEvents>(transaction);
        emit_event(
            &mut scheduled_transaction_events.transaction_execution_failed_events,
            TransactionExecutionFailedEvent {
                sender: scheduled_transaction.sender,
                payload: scheduled_transaction.payload,
                execution_error,
            }
        );
    }
}
