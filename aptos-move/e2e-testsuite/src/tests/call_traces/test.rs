use crate::tests::call_traces::{add_set_message_txn, call_set_message_txn};
use language_e2e_tests::current_function_name;
use language_e2e_tests::executor::FakeExecutor;

#[test]
fn call_traces_are_available() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create an account
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // publish the test module
    let (module, txn) = add_set_message_txn(&sender, 10);
    executor.execute_and_apply(txn);

    let the_number = 42;

    // call the test function
    let call_set_message_txn = call_set_message_txn(&sender, 11, vec![module.clone()], the_number);
    let output = executor.execute_and_apply(call_set_message_txn);

    let function_call_trace = output
        .call_traces()
        .iter()
        .find(|trace| trace.function.as_str() == "set_number");

    assert!(function_call_trace.is_some())
}
