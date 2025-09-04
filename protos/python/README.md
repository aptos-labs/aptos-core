# Velor Protos

This repository contains the protobuf definitions for Velor.

## Usage
Import generated classes like this:
```python
from velor_protos.velor.transaction.v1.transaction_pb2 import Transaction
```

Then use them like this:
```python
def parse(transaction: Transaction):
    # Parse the transaction.
```

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for more information.
