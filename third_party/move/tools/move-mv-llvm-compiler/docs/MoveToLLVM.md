# Move to llvm-ir compiler

Generates llvm-ir from move bytecode. It provides a bridge between move-bytecode and llvm.
It leverages the llvm’s Rust C bindings (llvm-sys) and can support all targets llvm supports.
For example, it can generate eBPF or executable binaries.

> move-mv-llvm-compiler : move-vm :: Docker : Virtual Machine

## Goals of the compiler

- Retargettable move to llvm compiler

## Non-goals

-

## Components

- move-ir translator
- move-native runtime
- testing
- FFI

## Translation

All integer arithmetic types are [checked](https://move-language.github.io/move/integers.html#arithmetic) for narrowing, overflow and undefined behavior at runtime.

## Optimizations

TODO:

## Calling convention

Calls and returns are described in the [move-book#calling](https://move-language.github.io/move/functions.html#calling). For the most part,
it is similar to Rust's calling convention. For example, varargs aren't supported.

Functions that return multiple values, use a second-class tuple-like expression to bind, return, and destructure multiple values.

On exit from a function, we generate LLVM IR to wrap them up into a struct, which is returned as a single IR value. Similarly, when a callee that returns such a value is used in an expression, we generate IR to extract each actual value from the struct. (See [PR#105](https://github.com/solana-labs/move/pull/105))

## Serialization/Deserialization byte order

Data passed to the entrypoint from the VM is serialized from structs in little-endian order.
The rbpf VM [supports little endian](https://github.com/qmonnet/rbpf/blob/main/src/ebpf.rs).

## Support for native function calls

Some functions do not have a body specified, and instead have the body provided by the VM. These are called [native functions](https://move-language.github.io/move/functions.html#native-functions). These functions as part of the [move-native](https://github.com/solana-labs/move/tree/llvm-sys/language/move-native) runtime shipped with the compiler.

- [abort](https://move-language.github.io/move/abort-and-assert.html)
- [return](https://move-language.github.io/move/functions.html#return-type)
- [assert](https://move-language.github.io/move/abort-and-assert.html)
- debug_print

## FFI

- Checks before and after calling external functions

## Type information

- For Debugging
- TypeInfo in the binary
- Format of typeinfo

## Testing

- ir tests
- runnable tests

## Project stages

### Stage 1

- Lower Move bytecode to llvm-ir
  - Move bytecode docs [[binary-format](https://github.com/solana-labs/move/blob/main/language/move-binary-format/src/file_format.rs#L1107), [spec](https://github.com/solana-labs/move/blob/main/language/documentation/spec/vm.md)]
- Encode function arguments, returns to communicate with outside contracts using Borsh
  - [Intro](https://hexdocs.pm/borsh_serializer/readme.html), [Borsh serialization](https://solanacookbook.com/guides/serialization.html#setting-up-for-borsh-serialization)
- Add debug info to types.
- Add [BTF](https://github.com/cilium/cilium/blob/master/Documentation/bpf.rst) types to provide runtime type information
- Add definitions to bytecode keywords
  - Simple operations can be codegen in llvm itself  but high level operation can be just calls.
  - E.g., Lowering BorrowField (a move bytecode instruction) can be just a function call (with right set of parameter) that does the borrow as well as does runtime checks
  - The definitions maybe put in a rust library that is linked to the executable (see stage2)
- Testing
  - Unit
  - Github Hooks Integration to run pre-merge tests
