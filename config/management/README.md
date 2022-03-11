# Aptos Management tools

The `aptos-management` crate provides a framework for building CLI tools for various
purposes.  The purpose of breaking these into multiple tools is to simplify the user
experience, and prevent confusion between the different use cases.

### The Tools
```
aptos-management
|-> aptos-genesis-tool = A tool for performing the genesis ceremony for the Aptos blockchain.
|-> aptos-operational-tool = A tool for performing management operations on the Aptos blockchain.
```

There are README's for each tool individually.
