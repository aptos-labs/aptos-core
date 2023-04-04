This is a small example of using the new `aptos` dependency. This shall be removed once we have
documentation/tests.

`pack2` contains a package which is used by `pack1` as follows:

```
[dependencies]
Pack2 = { aptos = "http://localhost:8080", address = "default" }
```

To see it working:

```shell
# Start a node with an account
aptos node run-local-testnet --with-faucet &
aptos account create --account default --use-faucet 
# Compile and publish pack2
cd pack2
aptos move compile --named-addresses project=default     
aptos move publish --named-addresses project=default
# Compile pack1 agains the published pack2
cd ../pack1
aptos move compile --named-addresses project=default     
```