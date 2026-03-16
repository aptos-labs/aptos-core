# Aptos Command Line Interface (CLI) Tool

The `aptos` tool is a command line interface (CLI) for debugging, development, and node operation.

See [Aptos CLI Documentation](https://aptos.dev/tools/aptos-cli/) for how to install the `aptos` CLI tool and how to use it.

## Config Encryption

The CLI can encrypt sensitive fields in `.aptos/config.yaml` (private keys, API keys, auth tokens) while keeping non-sensitive fields (network, URLs, public keys, account addresses) in plaintext.

### Enable encryption

```bash
aptos config encrypt
```

You'll be prompted to enter and confirm a password. The password is used to derive an AES-256-GCM key via Argon2id.

### Enable encryption with OS keyring caching

```bash
aptos config encrypt --use-keyring
```

This stores the password in your OS keyring (macOS Keychain, Windows Credential Manager, or Linux Secret Service) so you don't have to re-enter it for commands that need sensitive fields.

> **Note:** Keyring support requires the `keyring-cache` build feature. Pre-built releases include it for macOS and Windows. On Linux, install `libdbus-1-dev` (Debian/Ubuntu) or `dbus-devel` (Fedora) and build with `cargo build -p aptos --features keyring-cache`.

### Disable encryption

```bash
aptos config decrypt
```

This decrypts all fields, removes the `encryption:` section, and clears any stored keyring entry.

### Password entry

When a command needs to decrypt sensitive fields, the password is resolved in order:

1. `APTOS_CONFIG_PASSWORD` environment variable
2. OS keyring (if `--use-keyring` was used during encryption)
3. Interactive terminal prompt

Read-only commands like `aptos account balance` or `aptos account list` skip encrypted fields entirely and never prompt for a password.

### Changing the password

Decrypt first, then re-encrypt:

```bash
aptos config decrypt
aptos config encrypt
```
