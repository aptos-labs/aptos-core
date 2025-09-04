# Secure Coding for Velor Core

These Rust Secure Coding Guidelines are essential for anyone contributing to Velor, reflecting our security-first approach. As Velor is built with a primary focus on security, these guidelines, derived and adapted from ANSSI's Secure Rust Guidelines, are integral to maintaining the high standards of safety and robustness in velor-core. Velor contributors are encouraged to thoroughly understand and apply these principles in their work.

## Development Environment

### Rustup

Utilize Rustup for managing Rust toolchains. However, keep in mind that, from a security perspective, Rustup performs all downloads over HTTPS, but it does not yet validate signatures of downloads. Security is shifted to [crates.io](http://crates.io) and GitHub repository hosting the code [[rustup]](https://www.rust-lang.org/tools/install).

### Stable Toolchain

Velor Core leverages Rust stable toolchain to limit potential compiler, runtime, or tooling bugs, or potential supply chain attacks in nightly releases.

### Cargo

Utilize Cargo for project management without overriding variables like `debug-assertions` and `overflow-checks`.

- **`debug-assertions`**: This variable controls whether debug assertions are enabled. Debug assertions are checks that are only present in debug builds. They are used to catch bugs during development by validating assumptions made in the code.
- **`overflow-checks`**: This variable determines whether arithmetic overflow checks are performed. In Rust, when overflow checks are enabled (which is the default in debug mode), an integer operation that overflows will cause a panic in debug builds, preventing potential security vulnerabilities like buffer overflows.

### Linters and Formatters

Regularly use tools like Clippy and Rustfmt for identifying potential issues and maintaining code style. Velor **enforces** Clippy during automated testing with additional rules, so ensure to run it locally to prevent CI/CD failures.

Clippy with Velor-specific configuration can be run locally via `cargo xclippy` or using rust-analyser in your preferred IDE following these [instructions](https://rust-analyzer.github.io/manual.html#clippy). Velor uses directives in files and a per-directory configuration to turn on or off checks.

### Rustfix

Apply `rustfix` for compiler warnings and edition transitions, but verify the automatic fixes to ensure that the recommendations match the purpose of the code.

### Documentation

Document safety invariants and security considerations in code, especially for public and `unsafe` functions.

## Libraries

### Crate Quality and Security

Assess and monitor the quality and maintenance of crates that are being introduced to the codebase, employing tools like `cargo-outdated` and `cargo-audit` for version management and vulnerability checking.

- Velor utilizes **[Dependabot](https://github.com/dependabot)** to continuously monitor libraries. Our policy requires mandatory updates for critical and high-vulnerabilities, or upon impact evaluation given the context for medium and lower.
- We recommend leveraging [deps.dev](https://deps.dev) to evaluate new third party crates. This site provides an OpenSSF scorecard containing essential information. As a guideline, libraries with a score of 7 or higher are typically safe to import. However, those scoring **below 7** must be flagged during the PR and require a specific justification.

### Minimize Use of Feature Flags

As a general practice, avoid using feature flags in your crates unless absolutely necessary. Feature flags can introduce complexity and unexpected behaviours, making the codebase harder to audit for security vulnerabilities.

### Understanding Feature Unification

Be aware of Cargo's feature unification process. When multiple dependencies require the same crate with different feature flags, Cargo unifies these into a single configuration. This unification can inadvertently enable features that might not be desirable or secure for the project [[Rustbook: features unification]](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification) [[Rustbook: feature resolver]](https://doc.rust-lang.org/cargo/reference/features.html#feature-resolver-version-2).

## Language Generalities

### Unsafe Code

Never use `unsafe` blocks unless as a last resort. Justify their use in a comment, detailing how the code is effectively safe to deploy.

```rust
  foo(
      // SAFETY:
      // This is a valid safety comment
      unsafe { *x }
  )
```

```rust
  use std::ptr::NonNull;
  let a = &mut 42;

  // SAFETY: references are guaranteed to be non-null.
  let ptr = unsafe { NonNull::new_unchecked(a) };
```

### Integer Overflows

Refer to [coding-guidelines](./RUST_CODING_STYLE.md#integer-arithmetic).

### Error Handling

Use `Result<T, E>` and `Option<T>` for error handling instead of _unwrapping_ or _expecting_, to avoid panics, more details on [coding-style](./RUST_CODING_STYLE.md#error-handling).

### Assertions

Prefer using `Result` and context-rich error handling over Rust's `assert!`, `assert_eq!`, and `assert_ne!` macros for enforcing invariants, reserving assertions for development and unrecoverable error scenarios.

## Types Systems and Data Structures

### Drop Trait

Implement the `Drop` trait selectively, only when necessary for specific destructor logic. It's mainly used for managing external resources or memory in structures like Box or Rc, often involving unsafe code and security-critical operations.

In a Rust secure development, the implementation of the `std::ops::Drop` trait
must not panic.

Do not rely on `Drop` trait in security material treatment after the use, use [zeroize](https://docs.rs/zeroize/latest/zeroize/#) to explicit destroy security material, e.g. private keys.

### Send and Sync Traits

Be cautious with manual implementations of `Send` and `Sync` traits [[Rustbook: typesystem]](https://anssi-fr.github.io/rust-guide/06_typesystem.html#send-and-sync-traits) [[Rustbook: send and sync]](https://doc.rust-lang.org/nomicon/send-and-sync.html).
Both traits are _unsafe traits_, i.e., the Rust compiler does not verify in any way that they are implemented correctly. The danger is real: an incorrect implementation may lead to **undefined behavior**.

In the majority of scenarios, manual implementation is unnecessary. In Rust, nearly all primitive types intrinsically implement Send and Sync traits, and for a significant proportion of compound types, the Rust compiler automatically derives these implementations.

### Comparison Traits

Ensure the implementation of standard comparison traits respects documented invariants.
In the context of implementing standard comparison traits (like Eq, PartialEq, Ord, PartialOrd in Rust), respecting documented invariants means that the implementation of these traits should adhere to the properties and expectations defined by those invariants. For instance, if an invariant states that an object's identity is determined by certain fields, comparisons (equality, greater than, less than, etc.) must only consider those fields and ignore others. This ensures consistency, predictability, and correctness in how objects are compared, sorted, or considered equal within the Velor Core.

The ANSSI resource extensively covers the matter [References](#references).

### Enums

Prefer enums for state management to prevent invalid state representation.

### Concurrency Safe Primitives

Make use of Rust’s concurrency primitives like `Arc`, `Mutex`, and `RwLock` to manage shared state [[Rustbook: concurrency]](https://doc.rust-lang.org/book/ch16-00-concurrency.html).
By utilizing these primitives, Rust programs can manage shared resources among multiple threads safely and efficiently, adhering to Rust's goals of enabling fearless concurrency and memory safety. This is critical in multithread programs because mistakes in concurrency can be very costly, both for the system's stability and security.

### Data Structures with Deterministic Internal Order

Certain data structures, like HashMap and HashSet, do not guarantee a deterministic order for the elements stored within them. This lack of order can lead to problems in operations that require processing elements in a consistent sequence across multiple executions. In the Velor blockchain, deterministic data structures help in achieving consensus, maintaining the integrity of the ledger, and ensuring that computations can be reliably reproduced across different nodes.

Below is a list of deterministic data structures available in Rust. Please note, this list may not be exhaustive:

- **BTreeMap:** maintains its elements in sorted order by their keys.
- **BinaryHeap:** It maintains its elements in a heap order, which is a complete binary tree where each parent node is less than or equal to its child nodes.
- **Vec**: It maintains its elements in the order in which they were inserted. ⚠️
- **LinkedList:** It maintains its elements in the order in which they were inserted. ⚠️
- **VecDeque:** It maintains its elements in the order in which they were inserted. ⚠️

## Cryptography

### No Custom Cryptographic Algorithm

Use exclusively the cryptographic primitives exposed by the `velor-crypto` crate.

### Cryptographic Material Management

Adhere strictly to established protocols for generating, storing, and managing cryptographic keys. This includes using secure random sources for key generation, ensuring keys are stored in protected environments, and implementing robust management practices to handle key lifecycle events like rotation and revocation [Key Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html).

### Zeroing Sensitive Data

Use [zeroize](https://docs.rs/zeroize/latest/zeroize/#) for zeroing memory containing sensitive data.

## Misc

### Forget and Memory Leaks

Avoid using `std::mem::forget` in secure development, or any other function that leaks the memory.

Reference cycles can also cause memory leakage [[Rustbook: leak]](https://doc.rust-lang.org/book/ch15-06-reference-cycles.html?highlight=leak#reference-cycles-can-leak-memory).

Most memory leaks result in general product reliability problems. If an attacker can intentionally trigger a memory leak, the attacker might be able to launch a denial-of-service attack (by crashing or hanging the program).

### Fuzzing

Velor contains harnesses for fuzzing crash-prone code like deserializers, using [`libFuzzer`](https://llvm.org/docs/LibFuzzer.html) through [`cargo fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html). For more examples, see the `testsuite/fuzzer` directory where find detailed README.md.

## Conclusion

These guidelines are a crucial element for anyone contributing to Velor, reflecting our commitment to a security-first approach. By adhering to these guidelines, Velor contributors play a vital role in maintaining the security and robustness of the Velor network. As we work towards automating the enforcement of these standards, following these practices will help maintain and improve the overall integrity and resilience of the Velor ecosystem. This ongoing effort ensures that Velor continues to set a high bar for security and reliability.

## References

- ANSSI's Secure Rust Guidelines: https://anssi-fr.github.io/rust-guide/
