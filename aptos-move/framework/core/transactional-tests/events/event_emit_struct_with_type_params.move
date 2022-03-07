//# init --parent-vasps Alice

// TODO: this should be a Move test. Make it so after we fix the infrastructure.

//# publish
module Alice::M {
    use Std::Event;

    struct MyEvent<phantom T1, phantom T2> has copy, drop, store { b: bool }

    public fun emit_event<T1: copy + drop + store, T2: copy + drop + store>(account: &signer) {
        let handle = Event::new_event_handle<MyEvent<T2, T1>>(account);
        Event::emit_event(&mut handle, MyEvent{ b: true });
        Event::destroy_handle(handle);
    }
}

//# run --admin-script --signers DiemRoot Alice --show-events
script {
use Alice::M;

fun main(_dr: signer, account: signer) {
    M::emit_event<bool, u64>(&account);
}
}
