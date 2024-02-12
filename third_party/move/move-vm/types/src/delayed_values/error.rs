use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;

pub fn code_invariant_error<M: std::fmt::Debug>(message: M) -> PartialVMError {
    let msg = format!(
        "Delayed logic code invariant broken (there is a bug in the code), {:?}",
        message
    );
    println!("ERROR: {}", msg);
    PartialVMError::new(StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR).with_message(msg)
}
