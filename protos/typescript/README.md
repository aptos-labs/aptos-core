# Aptos Protos

This repository contains the protobuf definitions for the Aptos tech stack.

## Usage
Import the generated code like this:
```typescript
import { aptos } from "@aptos-labs/aptos-protos";
```

Then use it like this:
```typescript
function parse(transaction: aptos.transaction.v1.Transaction) {
  console.log(transaction)
}
```

These configuration options are required for typechecking to work:
```json
// tsconfig.json
{
  "compilerOptions": {
    "moduleResolution": "node",
  }
}
```

This package should work for both CommonJS (`"type": "commonjs"`) and ES (`"type": "module"`) modules.

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for more information.
