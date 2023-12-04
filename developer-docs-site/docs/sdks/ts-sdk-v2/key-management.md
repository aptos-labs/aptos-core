---
title: "Key Management"
---

The SDK provides an `Account` class for creating and managing [account](../../concepts/accounts.md) on Aptos network.

Following [AIP-55](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-55.md) the SDK supports `Legacy` and `Unified` authentications. `Legacy` includes `ED25519` and `MultiED25519` and `Unified` includes `SingleSender` and `MultiSender` authenticators.

- `SingleSender` supports any single signer authenticator (currently is `ED25519` and `Secp256k1`)
- `MultiSender` supports any multi signers authenticator (Currently is `MultiED25519`)

The `Account` class supports different static methods to generate and/or derive an account

- `Account.generate()`
- `Account.fromPrivateKey()`
- `Account.fromPrivateKeyAndAddress()`
- `Account.fromDerivationPath()`

### Generate a new account

To generate a new account (or a new key pair), the SDK provides a `generate()` static method on the `Account` class.

Account generation supports all current Aptos supported key schemes, `Legacy Ed25519`, `Single Sender Ed25519` and `Single Sender Secp256k1`.

```ts
const account = Account.generate(); // defaults to Legacy Ed25519
const account = Account.generate({ scheme: SingingSchemeInput.Secp256k1 }); // Single Sender Secp256k1
const account = Account.generate({ scheme: SingingSchemeInput.Ed25519, legacy: false }); // Single Sender Ed25519
```

:::note
Creating an account with the SDK creates it locally, to create the account on chain we should fund it.

```ts
const transaction = await aptos.fundAccount({ accountAddress: account.accountAddress, amount: 100 });
```

:::

### Derive an account from private key

The SDK supports deriving an account from a private key with `fromPrivateKey()` static method.
This method uses a local calculation and therefore is used to derive an `Account` that has not had its authentication key rotated.

```ts
// to derive an account with a legacy Ed25519 key scheme
const privateKey = new Ed25519PrivateKey(privateKeyBytes);
const account = Account.fromPrivateKey({ privateKey });

// to derive an account with a Single Sender Ed25519 key scheme
const privateKey = new Ed25519PrivateKey(privateKeyBytes);
const account = Account.fromPrivateKey({ privateKey, legacy: false });

// to derive an account with a Single Sender Secp256k1 key scheme
const privateKey = new Secp256k1PrivateKey(privateKeyBytes);
const account = Account.fromPrivateKey({ privateKey });
```

### Derive an account from private key and address

The SDK supports deriving an account from a private key and address with `fromPrivateKeyAndAddress()` static method.

```ts
// to derive an account with a legacy Ed25519 key scheme
const privateKey = new Ed25519PrivateKey(privateKeyBytes);
const accountAddress = AccountAddress.from(address);
const account = Account.fromPrivateKeyAndAddress({ privateKey, address: accountAddress, legacy: true });

// to derive an account with a Single Sender Ed25519 key scheme
const privateKey = new Ed25519PrivateKey(privateKeyBytes);
const accountAddress = AccountAddress.from(address);
const account = Account.fromPrivateKeyAndAddress({ privateKey, address: accountAddress, legacy: false });

// to derive an account with a Single Sender Secp256k1 key scheme
const privateKey = new Secp256k1PrivateKey(privateKeyBytes);
const accountAddress = AccountAddress.from(address);
const account = Account.fromPrivateKeyAndAddress({ privateKey, address: accountAddress });
```

### Derive an account from derivation path

The SDK supports deriving an account from derivation path with `fromDerivationPath()` static method.

```ts
// to derive an account with a legacy Ed25519 key scheme
const { mnemonic, address, path } = wallet;
const acccount = Account.fromDerivationPath({
  path,
  mnemonic,
  scheme: SigningSchemeInput.Ed25519,
});

// to derive an account with a Single Sender Ed25519 key scheme
const { mnemonic, address, path } = wallet;
const acccount = Account.fromDerivationPath({
  path,
  mnemonic,
  scheme: SigningSchemeInput.Ed25519,
  legacy: false,
});

// to derive an account with a Single Sender Secp256k1 key scheme
const { mnemonic, address, path } = wallet;
const acccount = Account.fromDerivationPath({
  path,
  mnemonic,
  scheme: SigningSchemeInput.Secp256k1Ecdsa,
});
```
