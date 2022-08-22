**NOTE**: These examples are tested to work with the SDK in the parent
directory, meaning these examples on a given commit X are only tested
against the SDK from commit X. Therefore, there is no guarantee that
these examples work with devnet. As such, to test these examples, you
must run a local testnet, something like this:
```
cargo run -p aptos -- node run-local-testnet --with-faucet --faucet-port 8081 --force-restart --assume-yes
```

Before running these examples, make sure to run the following in the parent
directory (`ecosystem/typescript/sdk`). This is necessary because the tests
in the example import the code directly from the parent, rather than from
npm.js, to ensure that code is matching when being tested.

```
yarn install
yarn build
```

Every time you change the SDK, you must re-run the above steps and then run
this in each of the examples directories:
```
rm yarn.lock && yarn install
```

