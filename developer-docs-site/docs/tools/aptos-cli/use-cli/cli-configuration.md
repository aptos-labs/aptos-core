---
title: "Configuration"
id: "cli-configuration"
---


## Configuration examples

Configuration for the CLI works like this:

### In the current working directory for local runs

1. Your configurations are in a **local** YAML configuration file `.aptos/config.yaml`, i.e., located in the current working directory where you run the CLI. In this case you must run your CLI commands from this current working directory for this configuration to be used.
2. You can verify that the CLI is set to use this local configuration YAML file by running the command:

```bash
aptos config show-global-config
```

You should see the below output:

```bash
{
  "Result": {
    "config_type": "Workspace"
  }
}
```

The `Workspace` value for the `config_type` indicates that the `.aptos/config.yaml` file is used for the CLI configuration.

### In the home directory for the global runs

1. Your configurations are in a **global** YAML configuration file `~/.aptos/global_config.yaml`, i.e., located in your home directory.
2. Set the CLI to use this global configuration YAML file by running this command:

```bash
aptos config set-global-config --config-type global
```

You will see the below output:

```
{
  "Result": {
    "config_type": "Global"
  }
}
```

You can also show the global configuration with the `show-global-config` command.

```bash
$ aptos config show-global-config
{
  "Result": {
    "config_type": "Global"
  }
}
```

:::tip Default configuration
If you did not set any global configuration, then the `./.aptos/config.yaml` in the current working directory is used for configuration.
:::

### Setting up shell completion

You can set up shell completions with the `generate-shell-completions` command. You can lookup configuration for your specific shell. The supported shells are `[bash, zsh, fish, powershell, elvish]`. An example is below for [`oh my zsh`](https://ohmyz.sh/).

```bash
aptos config generate-shell-completions --shell zsh --output-file ~/.oh-my-zsh/completions/_aptos
```

## Initialize local configuration and create an account

A local folder named `.aptos/` will be created with a configuration `config.yaml` which can be used to store configuration between CLI runs. This is local to your run, so you will need to continue running CLI from this folder, or reinitialize in another folder.

### Step 1: Run Aptos init

The `aptos init` command will initialize the configuration with the private key you provided.
Note: If you would like to initialize a new profile from ledger, please refer to the [Ledger documentation](./use-aptos-ledger.md).

```bash
$ aptos init
Configuring for profile default
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696!  Run `aptos help` for more information about commands

{
  "Result": "Success"
}
```

### Step 2: Changing the configuration

To change the configuration, you can either run the command `aptos init` or you can manually edit the `.aptos/config.yaml` that is in your current working directory.

### Creating other profiles

You can also create other profiles for different endpoints and different keys. These can be made by adding the `--profile` argument, and can be used in most other commands to replace command line arguments.

```bash
$ aptos init --profile superuser
Configuring for profile superuser
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```