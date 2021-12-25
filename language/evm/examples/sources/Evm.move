/// Module which provides access to EVM functionality, including information about the executing transaction.
////
/// This currently only represents a basic subset of what we may want to expose.
module Evm::Evm {

    /// Returns the address of the executing contract.
    public native fun self(): address;

    /// Returns the address of the transaction sender.
    public native fun sender(): address;

    /// If this is a payable transaction, returns the value (in Wei) associated with it.
    /// TODO: need u256
    public native fun value(): u128;

    /// Returns the balance, in Wei, of any account.
    public native fun balance(addr: address): u128;

    /// Transfers the given amount to the target account.
    public native fun transfer(addr: address, amount: u128);

    /// Emits an event. The type passed for `E` must be annotated with #[event].
    public native fun emit<E>(e: E);

    /// Creates a signer for the contract's address space.
    public native fun sign(addr: address): &signer;
}
