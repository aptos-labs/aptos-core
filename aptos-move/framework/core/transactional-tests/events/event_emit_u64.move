//# init --parent-vasps Alice

// TODO: this should be a Move test. Make it so after we fix the infrastructure.

//# run --admin-script --signers DiemRoot Alice --show-events
script {
use Std::Event;

fun main(_dr: signer, account: signer) {
    let handle = Event::new_event_handle<u64>(&account);
    Event::emit_event(&mut handle, 42);
    Event::destroy_handle(handle);
}
}
