---
title: "Key"
id: "cli-key"
---

## Key examples

### Generating a key

To allow generating private keys, you can use the `aptos key generate` command. You can generate
either `x25519` or `ed25519` keys.

```bash
$ aptos key generate --key-type ed25519 --output-file output.key
{
  "Result": {
    "PrivateKey Path": "output.key",
    "PublicKey Path": "output.key.pub"
  }
}
```

### Generating a vanity prefix key

If you are generating an `ed25519` key, you can optionally supply a vanity prefix for the corresponding account address:

```bash
$ aptos key generate --output-file starts_with_ace.key --vanity-prefix 0xace
{
  "Result": {
    "PrivateKey Path": "starts_with_ace.key",
    "PublicKey Path": "starts_with_ace.key.pub",
    "Account Address:": "0xaceffa015e51dcd32c34794c143e19185b3f1be5464dd6184239a37e57e72ea3"
  }
}
```

This works for multisig accounts too:

```bash
% aptos key generate --output-file starts_with_bee.key --vanity-prefix 0xbee --vanity-multisig
{
  "Result": {
    "PrivateKey Path": "starts_with_bee.key",
    "PublicKey Path": "starts_with_bee.key.pub",
    "Account Address:": "0x384cf987aab625f9727684d4dda8de668abedc18aa8dceabd7651a1cfb69196f",
    "Multisig Account Address:": "0xbee0797c577428249125f6ed7f4a2a5939ddc34389294bd9f5d1627508832f56"
  }
}
```

Note the vanity flag documentation from the `aptos key generate` help:

```
--vanity-multisig
    Use this flag when vanity prefix is for a multisig account. This mines a private key for
    a single signer account that can, as its first transaction, create a multisig account
    with the given vanity prefix

--vanity-prefix <VANITY_PREFIX>
    Vanity prefix that resultant account address should start with, e.g. 0xaceface or d00d.
    Each additional character multiplies by a factor of 16 the computational difficulty
    associated with generating an address, so try out shorter prefixes first and be prepared
    to wait for longer ones
```

:::tip
If you want even faster vanity address generation for long prefixes, try out the parallelism-optimized [`optivanity`](https://github.com/econia-labs/optivanity) tool from [Econia Labs](https://www.econialabs.com/)
:::

### Generating a peer config

To allow others to connect to your node, you need to generate a peer configuration. Below command shows how you can use
the `aptos` CLI to generate a peer configuration and write it into a file named `peer_config.yaml`.

```bash
$ aptos key extract-peer --output-file peer_config.yaml
```

The above command will generate the following output on the terminal:

```bash
{
  "Result": {
    "8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752": {
      "addresses": [],
      "keys": [
        "0x8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752"
      ],
      "role": "Upstream"
    }
  }
}
```

The `peer_config.yaml` file will be created in your current working directory, with the contents as shown in the below example:

```bash
---
8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752:
  addresses: []
  keys:
    - "0x8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752"
  role: Upstream
```

**Note:** In the addresses key, you should fill in your address.