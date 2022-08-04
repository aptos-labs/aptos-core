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

