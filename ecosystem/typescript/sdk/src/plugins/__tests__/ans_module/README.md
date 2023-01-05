### Generate ANS module for testing

#### Why doing it this way?

While running tests we spin up a local testnet to test against it. For framework modules (0x1,0x2,0x3) it is easier to test as it is part of the local testnet. For packages in custom addresses, we need to compile and publish the package on a given address and then test against it. ANS is one example for this kind of packages.
Until we have a better tool/solution to compile a move package in TS, we need to compile the package locally using CLI, and then we can publish it using the ts sdk during tests.

ANS compiled branch = main
Latest ANS compiled version = 1.0.0
Latest ANS compiled commit = 143bdb52281ba5bc4b4f0a12b84dd0f3f06c7b9b

1. cd into `core` in [this](https://github.com/aptos-labs/aptos-names-contracts) repo
2. Create a new account so you have access to the account's private key, can generate an account with

- Aptos CLI - `aptos init --network local`
- Create a new account on wallet (Petra) under the `local` network (you would need to run a local node for that)

3. Replace `aptos_names`, `aptos_names_admin`, `aptos_names_funds` with the generated acconunt address on step #2.

4. Compile the ANS package with Aptos CLI:
   run the compile command, and use the account address from step #2.

Make sure you compile the package with an account address you have access to its private key,

```
aptos move compile --save-metadata --named-addresses aptos_names=<address>,aptos_names_admin=<address>,aptos_names_funds=<address>
```

That generates the package files, copy `package-metadata.bcs` file and `bytecode_modules` folder to this folder (`ans_modules/`).

On `ans_client.test.ts` -

- Change `ans_owner_address` to the new account address
- Change `const owner` to use the new account private key (replace the hex string passed to `AptosAccount`)
