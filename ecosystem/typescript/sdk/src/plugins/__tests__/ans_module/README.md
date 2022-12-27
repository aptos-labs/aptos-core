### Generate ANS module for testing

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
