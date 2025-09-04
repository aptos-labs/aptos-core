This guide gives some hints to troubleshoot issues when using the prover for specifying the Velor frameworks.

## Installation

Please refer to the [doc](https://velor.dev/tools/velor-cli/install-cli/install-move-prover/).

## Timeout

When the prover cannot finish the verification job within a specified time (by default 40s), it will exit and generate an error message.
In this case, users should add a pragma `pragma verify = false` to the specification
that causes the timeout with a `TODO` comment for the prover developer to debug, as shown in the example.

```move
spec foo {
   pragma verify = false; // TODO: set to false because of timeout
}
```

## Internal errors

Bugs in the prover often lead to `boogie internal errors`. When it happens, you could try to locate the specs that causes this issue and comment them out. 
If the error is caused by the Move code, e.g., `foo.move`, You could add the following code in `foo.spec.move` (create one if it does not exist) with 
a `TODO` comment preferably containing the URL to the corresponding Github issue.

```move
spec module {
   pragma verify = false; // TODO: see issue <url>
}
```
After making these changes, please submit a Github issue for the prover team to fix.

## Suppressing prover tests

Prover tests are land-blockers for PRs which change the Move code and/or specifications in the `framework` directory. To disable them locally for efficiency,
you could use the command `cargo test --release -p velor-framework -- --skip prover`.


## Specification guide

Please refer to the [Move Prover Book](https://velor.dev/guides/prover-guides/move-prover)
for detailed introduction on how to write specifications and use Move Prover.
