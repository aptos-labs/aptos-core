This is a small example of using the new `velor` dependency. This shall be removed once we have
documentation/tests.

`pack2` contains a package which is used by `pack1` as follows:

```
[dependencies]
Pack2 = { velor = "http://localhost:8080", address = "default" }
```

To see it working:

```shell
# Start a node with an account
velor node run-local-testnet &
velor account create --account default --use-faucet 
# Compile and publish pack2
cd pack2
velor move compile --named-addresses project=default     
velor move publish --named-addresses project=default
# Compile pack1 agains the published pack2
cd ../pack1
velor move compile --named-addresses project=default     
```
