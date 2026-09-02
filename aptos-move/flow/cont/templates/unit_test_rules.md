{# Move unit testing rules #}
{% if once(name="unit_test_rules") %}

## Move unit-test syntax

### Test Attributes

| Attribute | Usage |
|-----------|-------|
| `#[test]` | Marks a function as a test |
| `#[test(name = @addr)]` | Test with signer parameters bound to addresses |
| `#[test_only]` | Code only compiled for testing (modules, functions, structs, constants) |
| `#[expected_failure]` | Test expected to abort (any code) |
| `#[expected_failure(abort_code = N)]` | Expects abort with specific code |
| `#[expected_failure(abort_code = N, location = mod)]` | Expects abort at specific location |

### Signer parameters

Signers are bound via the test attribute, not passed as arguments:

```move
// Single signer
#[test(account = @0x1)]
fun test_single(account: &signer) { }

// Multiple signers
#[test(admin = @admin_addr, user = @0x42)]
fun test_multi(admin: &signer, user: &signer) { }

// Framework signer for timestamp, account creation, etc.
#[test(aptos_framework = @aptos_framework)]
fun test_framework(aptos_framework: &signer) { }
```

### Expected failures

Use `#[expected_failure]` to test that code correctly aborts under certain conditions.

Prefer an exact code and location. Use an unconstrained expected failure only
when the failure kind genuinely is the behavior under test.

**Any abort:**
```move
#[test]
#[expected_failure]
fun test_will_abort() { abort 1 }
```

**With abort code** (must match exactly):
```move
#[test]
#[expected_failure(abort_code = E_NOT_AUTHORIZED, location = Self)]
fun test_unauthorized() { /* should abort with E_NOT_AUTHORIZED */ }
```

**With location** (when error originates in another module):
```move
#[test]
#[expected_failure(abort_code = 26113, location = extensions::table)]
fun test_table_error() { /* should abort in table module */ }
```

**Execution errors** (not abort, but runtime failures):
```move
// Arithmetic error (overflow, divide by zero)
#[test]
#[expected_failure(arithmetic_error, location = Self)]
fun test_overflow() { let _ = 255u8 + 1; }

// Vector out of bounds
#[test]
#[expected_failure(vector_error, minor_status = 1, location = Self)]
fun test_out_of_bounds() { vector::borrow(&vector::empty<u8>(), 0); }
```

### Test-only code

```move
// Test-only module (entire module only compiled for tests)
#[test_only]
module my_addr::test_helpers {
    public fun setup(): u64 { 100 }
}

// Test-only function in regular module
module my_addr::my_module {
    #[test_only]
    public fun init_for_testing(account: &signer, value: u64) {
        move_to(account, MyResource { value });
    }
}
```

### Common test utilities

```move
// Get address from signer
use std::signer;
let addr = signer::address_of(account);

// Create account (registers on-chain)
use aptos_framework::account;
account::create_account_for_test(addr);

// Create signer without registering (lightweight)
let signer = account::create_signer_for_test(@0x123);

// Check resource exists
assert!(exists<MyResource>(addr), E_NOT_FOUND);

// Initialize timestamp (required before time functions)
use aptos_framework::timestamp;
timestamp::set_time_has_started_for_testing(aptos_framework);

// Advance time
timestamp::update_global_time_for_test(1000000); // microseconds
timestamp::update_global_time_for_test_secs(100); // seconds
```

## Test design

- Test one behavior per case with the minimum setup needed to reach it.
- Call the requested function directly when its visibility permits. For an
  inaccessible private function, test it through observable public behavior or
  use an existing module-local test pattern; do not change production
  visibility merely for a test.
- Explain the behavior being checked, especially for abort and boundary cases.
- Assert semantic outcomes, not incidental implementation details.

**Naming:** `test_<function>_<scenario>` (e.g., `test_transfer_insufficient_balance`), module: `<module>_tests`

**Common mistakes:**
- `RESOURCE_ALREADY_EXISTS`: Don't initialize the same resource twice
- `MISSING_DATA`: Ensure required resources exist before calling
- Signer mismatch: Operations checking `signer::address_of()` require the correct signer. Match signers to expected addresses in test attributes.

{% endif %}
