# API

This module provides REST API for client applications to query the Diem blockchain.

The [API specification](blueprint.apib) is documented in [API Blueprint](https://apiblueprint.org) format.

## Testing

### Integration/Smoke Test

Run integration/smoke tests in `testsuite/smoke-test`

```
cargo test --test "forge" "api::"
```

### API Specification Test

* Build diem-node: `cargo build -p diem-node`
* Install [dredd](https://dredd.org/en/latest/)
* Run `dredd` inside 'api' directory.


### Render API into HTML Document


For example, use [snowboard](https://github.com/bukalapak/snowboard)

```
npm install -g snowboard
snowboard http blueprint.apib
open http://localhost:8088
```
