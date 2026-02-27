{# Move unit testing reference #}

## Move Unit Testing Reference

### Test Attributes

| Attribute | Usage |
|-----------|-------|
| `#[test]` | Marks a function as a test |
| `#[test(name = @addr)]` | Test with signer parameters bound to addresses |
| `#[test_only]` | Code only compiled for testing (modules, functions, structs) |
| `#[expected_failure]` | Test expected to abort (any code) |
| `#[expected_failure(abort_code = N)]` | Expects abort with specific code |
| `#[expected_failure(abort_code = N, location = mod)]` | Expects abort at specific location |

### Signer Parameters

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

### Expected Failure Variants

```move
// Specific abort code
#[expected_failure(abort_code = 42)]

// Abort code with location (when error originates in another module)
#[expected_failure(abort_code = 1, location = other_module)]

// Arithmetic error (overflow, divide by zero)
#[expected_failure(arithmetic_error, location = Self)]

// Vector index out of bounds
#[expected_failure(vector_error, location = std::vector)]
```

### Test-Only Code

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

### Useful Test Utilities

```move
// Get address from signer
use std::signer;
let addr = signer::address_of(account);

// Check resource exists
assert!(exists<MyResource>(addr), E_NOT_FOUND);

// Framework utilities
use aptos_framework::timestamp;
timestamp::set_time_has_started_for_testing(aptos_framework);

use aptos_framework::account;
account::create_account_for_test(addr);
```

