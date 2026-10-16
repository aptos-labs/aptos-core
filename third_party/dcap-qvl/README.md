# dcap-qvl

[![CI](https://github.com/Phala-Network/dcap-qvl/actions/workflows/rust.yml/badge.svg)](https://github.com/Phala-Network/dcap-qvl/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/dcap-qvl.svg)](https://crates.io/crates/dcap-qvl)
[![docs.rs](https://img.shields.io/docsrs/dcap-qvl)](https://docs.rs/dcap-qvl)
[![PyPI](https://img.shields.io/pypi/v/dcap-qvl)](https://pypi.org/project/dcap-qvl/)
[![npm](https://img.shields.io/npm/v/@phala/dcap-qvl)](https://www.npmjs.com/package/@phala/dcap-qvl)
[![Maven Central](https://img.shields.io/maven-central/v/com.phala/dcap-qvl-android)](https://central.sonatype.com/artifact/com.phala/dcap-qvl-android)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Verify Intel SGX and TDX attestation quotes.** `dcap-qvl` is a small, pure-Rust
library that verifies DCAP (Data Center Attestation Primitives) quotes, with
native bindings for Python, Go, Kotlin, and Swift — plus a standalone
pure-JavaScript implementation for the web.

## What it does

- **Verifies SGX and TDX quotes** against Intel's trust chain.
- **Gets collateral for you** from a PCCS or Intel PCS — or verifies fully
  **offline** with collateral you already have.
- **Extracts report fields** from a quote: measurements, report data, TCB
  status, and advisory IDs.
- **Runs everywhere.** The pure-Rust core works on servers, mobile,
  WebAssembly, and on-chain (`no_std`).

By default it uses Phala Network's PCCS (`https://pccs.phala.network`) for
better availability and lower rate limits.

## Languages

| Language | Package | Install | Guide |
|---|---|---|---|
| **Rust** | [`dcap-qvl`](https://crates.io/crates/dcap-qvl) | `cargo add dcap-qvl` | [docs.rs](https://docs.rs/dcap-qvl) |
| **Python** | [`dcap-qvl`](https://pypi.org/project/dcap-qvl/) | `pip install dcap-qvl` | [python-bindings/](python-bindings/) |
| **JavaScript** (pure JS) | [`@phala/dcap-qvl`](https://www.npmjs.com/package/@phala/dcap-qvl) | `npm i @phala/dcap-qvl` | [dcap-qvl-js/](dcap-qvl-js/) |
| **Go** | `github.com/Phala-Network/dcap-qvl/golang-bindings` | `go get github.com/Phala-Network/dcap-qvl/golang-bindings` | [golang-bindings/](golang-bindings/) |
| **Android** (Kotlin/Java) | [`com.phala:dcap-qvl-android`](https://central.sonatype.com/artifact/com.phala/dcap-qvl-android) | Gradle | [dcap-qvl-mobile/android/](dcap-qvl-mobile/android/) |
| **Swift** (iOS/macOS) | [`dcap-qvl-swift`](https://swiftpackageindex.com/Phala-Network/dcap-qvl-swift) | SwiftPM | [dcap-qvl-mobile/ios/](dcap-qvl-mobile/ios/) |

The JavaScript package is a standalone pure-JS port that runs in Node and the
browser with no native dependencies. WebAssembly builds of the Rust core are
also published as [`@phala/dcap-qvl-web`](https://www.npmjs.com/package/@phala/dcap-qvl-web)
and [`@phala/dcap-qvl-node`](https://www.npmjs.com/package/@phala/dcap-qvl-node).

There's also a command-line tool, [`dcap-qvl-cli`](cli/) (`cargo install dcap-qvl-cli`).

## How verification works

To verify a quote you need three things:

1. the **quote** bytes,
2. the **collateral** for it — the certificates, CRLs, and TCB info that prove
   the quote against Intel's trust chain, served by a PCCS, and
3. the **current time**, to check the collateral hasn't expired.

`verify()` checks the signature chain and TCB status and returns the report plus
a status string. If you don't already have collateral, the library can fetch it
from a PCCS for you in one step.

## Quick start

### Rust

```rust
use dcap_qvl::collateral::CollateralClient;
use dcap_qvl::verify::verify;
use dcap_qvl::PHALA_PCCS_URL;

let quote = std::fs::read("quote.bin")?;
let collateral = CollateralClient::with_default_http(PHALA_PCCS_URL)?
    .fetch(&quote)
    .await?;
let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)?
    .as_secs();
let report = verify(&quote, &collateral, now)?;
println!("status = {}", report.status);
```

See the [docs.rs page](https://docs.rs/dcap-qvl) for crypto-backend selection,
feature flags, `no_std`, and offline verification.

### Python

```python
import asyncio, dcap_qvl

async def main():
    quote = open("quote.bin", "rb").read()
    result = await dcap_qvl.get_collateral_and_verify(quote)  # defaults to Phala PCCS
    print(result.status)

asyncio.run(main())
```

### JavaScript

```javascript
import { getCollateralAndVerify } from '@phala/dcap-qvl';

const result = await getCollateralAndVerify(quoteBuffer); // defaults to Phala PCCS
console.log(result.status);
```

### Android (Kotlin)

```kotlin
import com.phala.dcapqvl.*

// collateralJson is the raw PCCS response body — fetch it with OkHttp / Ktor.
val report = verify(rawQuote, collateralJson, nowSecs)
println(report.status)
```

### Swift

```swift
import DcapQvl

let report = try verify(rawQuote: rawQuote, collateralJson: collateralJson, nowSecs: nowSecs)
print(report.status)
```

### Go

```go
import dcap "github.com/Phala-Network/dcap-qvl/golang-bindings"

report, _ := dcap.GetCollateralAndVerify(rawQuote, dcap.PhalaPCCSURL)
fmt.Println(report.Status)
```

Each binding's directory (linked in the table above) has full documentation and
runnable examples.

## Detailed claims and optional policy validation

Applications can request detailed serializable claims and optionally apply the built-in policy:

```rust
use dcap_qvl::{QuotePolicy, TcbStatus};
use dcap_qvl::verify::QuoteVerifier;

let policy = QuotePolicy::strict(now)
    .allow_status(TcbStatus::SWHardeningNeeded)
    .reject_advisory("INTEL-SA-00334");

let claims = QuoteVerifier::new_prod()
    .verify_with_policy(&quote, collateral, now, &policy)?;
```

For downstream appraisal, use `QuotePolicy::claims_only(now)`. It skips local business-policy checks while verification still uses the explicit `now` argument. Applications can serialize those claims and feed them to a
shared downstream policy engine that also handles other TEE platforms. See
[docs/policy.md](docs/policy.md) for the claims schema and `QuotePolicy`.
Existing one-shot `verify()` entry points remain available.

## Building and testing

Common tasks are in the [Makefile](Makefile):

```bash
cargo test              # test the Rust core
make build_python       # build + test the Python bindings
make test_wasm          # test the WASM packages
make build_mobile_android  # build the Android AAR
make build_mobile_ios      # build the iOS XCFramework (macOS only)
```

## Releasing

A single `v<X.Y.Z>` tag publishes every ecosystem at one version — crates.io,
PyPI, npm, Maven Central, and the Swift Package Index. See
[dcap-qvl-mobile/RELEASING.md](dcap-qvl-mobile/RELEASING.md).

## License

MIT — see [LICENSE](LICENSE).
