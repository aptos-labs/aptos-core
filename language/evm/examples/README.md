**Work in Progress**

This directory contains (a growing set of) examples of "Move-on-EVM", a programming model in Move for EVM.

- [Token.move](./sources/Token.move) contains an implementation of ERC20 which is intended to be compliant and callable
  from other EVM code (Move or otherwise).
- [Faucet.move](./sources/Faucet.move) contains the faucet example from the Ethereum book.

The basic programming model is as follows:

- The module [Evm.move](./sources/Evm.move) contains the API of a Move contract to the EVM. It encapsulates access to
  the transaction context and other EVM builtins.
- In the current model, *each Move EVM contract has its own isolated address space*. That is, `borrow_global` et. al
  work on memory private to this contract. This reflects the setup of the EVM most naturally, where storage between
  contracts cannot be shared apart from via accessor contract functions.
- In order to allow a contract to store to its own private memory, there is currently a pseudo function
  `Evm::sign(addr)` which allows a contract to convert any address into a signer for its own private memory. Eventually,
  we may want to remove the requirement to have a signer for move_to in the EVM context.
- Move EVM contracts use attributes to indicate the usage of structs for storage and events, and for functions to be
  callable from other contracts. It is expected that there is some codegen of Move from these attributes. Specifically,
  functions marked as `callable` have a generated API for cross-contract EVM call and delegate invocations.
