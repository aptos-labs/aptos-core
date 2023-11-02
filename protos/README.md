# Protos
This directory contains the protobuf definitions for all Aptos services. For the sake of simplifying release and minimizing potential version conflicts, we include all protos and code generated from those protos in one place.

If you update the proto definitions in `proto/`, you can regenerate the code for all languages based on those protos by running this script:
```bash
./scripts/build_protos.sh
```

If you haven't installed deps yet, run this script from this directory:
```bash
./scripts/install_deps.sh
```

Also make sure to install buf, e.g. for Mac:
```
brew install bufbuild/buf/buf
```
