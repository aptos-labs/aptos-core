# Language feature idea graveyard

Move is a smart contract language that counts minimalism as one of its guiding principles. Features are often proposed, but not always implemented. This list exists to record feature proposals that did not progress to acceptance. The goal of maintaining this list is to avoid duplicate proposals, and to explain the rationale for not moving forward with an idea that might seem promising at first blush.

## Pure data structs

### Decision
We should let adapters decide how to handle this

### Rationale
Structs being passed to transactions cause safety issues because deserialization can circumvent modular encapsulation. For example, below make is the only way to create S and the program guarantees that field x is always greater zero; however, if S is obtained via deserialization, this constraint can be violated::
```
struct S has drop, copy { x: u64 }
public fun make(x: u64) { assert!(x > 0); S{x} }

fun(entry) foo(s: S/*no guarantee about s.x*/) { … }
```

## Native structs

### Decision
This is obsolete as regular structs can be used from native code, e.g. struct S { native_handle: u128 }

### Rationale
As Move evolves to an extensible language, native structs are an important  tool for extensibility. However, support for native structs has been removed from the VM (though still supported in compiler and file format).

### Design
Bring back support for native structs in the VM. They can use a simple uniform representation via handles which are managed by the runtime environment (like u128 or u256 for each and every native struct value), so implementation effort should be small.

## Signer relaxation for `move_to`

### Decision
Tables and framework-specific workarounds like `ResourceAccount` make this unneccessary.

### Rationale
In some usage scenarios of Move, it does not make sense to only allow move_to<R>(s, x) with s a signer. For instance, when Move is running on the EVM, storage is not owned and paid for by the account owner, but by the contract caller, which manages the accounts on behalf of owners. In general, the signer requirement allows one to only create new resources at addresses for which the transaction has a signer, which disables the capability to manage resource collections on behalf of others.

## Design (Option A)
Drop the requirement of passing a signer to move-to all together. For downwards compatibility reasons, the compiler allows passing either a signer or an address. Rationale for this quite radical step is that in-real world contracts (e.g. the Diem Payment Network) more complex access control patterns than enabled by signer/move_to are needed anyway. Moreover, since only modules which declare a resource R can call move_to<R>, preventing publishing without a signer can still be modeled by modular encapsulation.

### Design (Option B)
Use an attribute on struct declarations to indicate a signer is not required for publishing this struct.

### Design (Option C)
Have another function which does not require a signer, e.g. move_to_address. This, however, raises questions of adequate naming and methodology (when to use which function, should there be constraints when which function is allowed, etc.)

### Design (Option D)
Add no new features and ask clients that want this feature (e.g., the Move -> EVM compilation effort) to implement it via design patterns and/or native functions. See SignerCapability in the Starcoin Account code for one example of a design pattern for (effectively) implementing move_to_address without changing the language.

### Design (Option E)
Support this via a new internal ability that allows framework designers to implement more powerful native functions with the same encapsulation guarantees as bytecodes. For example, the framework designer can define a function `public native fun move_to_address<T: internal>(a: address, t: T)` that can only be invoked on `T`’s declared in the calling module.

### Design (Option F)
Use tables when there is a need to index storage without signer.
