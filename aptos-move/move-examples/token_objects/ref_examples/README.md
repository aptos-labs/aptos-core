# Object Ref Examples
These examples are intended to demonstrate how to use the various `Refs` in the `object.move` module.

Ensure you have the Aptos CLI installed, and you are in the `ref_examples` folder in your terminal.

## Compile, test, and publish the module
### Initialize an Aptos `default` profile if you haven't yet
aptos init --profile default

### Compile the module
aptos move compile --named-addresses ref_examples=default,admin=default

### Run the module's unit tests
```shell
aptos move test --named-addresses ref_examples=default,admin=default
```

### Publish the module
```shell
aptos move publish --named-addresses ref_examples=default,admin=default
```
If you want to publish the module with a different @admin, just change `admin=default` in the command.

Upon publishing, the module automatically runs its `init_module` function, which means you'll have created the collection with the caller!
