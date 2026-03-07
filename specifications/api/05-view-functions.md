# View Functions API Specification

> **Version:** 1.0.0  
> **Last Updated:** January 28, 2026

## Overview

The View Functions API allows you to execute read-only Move functions without submitting a transaction. This is useful for querying on-chain state, computing values, and testing function outputs.

## Key Characteristics

- **Read-only**: Cannot modify blockchain state
- **No gas fees**: Free to call (no transaction required)
- **Simulated execution**: Runs against current or historical state
- **Type-safe**: Returns strongly typed Move values

## Endpoint

```
POST /v1/view
```

## Request Format

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `function` | string | Yes | Fully qualified function name |
| `type_arguments` | string[] | No | Generic type parameters |
| `arguments` | any[] | Yes | Function arguments |

### Query Parameters

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `ledger_version` | u64 | latest | Version to query against |

## Request Body

```json
{
  "function": "0x1::coin::balance",
  "type_arguments": ["0x1::aptos_coin::AptosCoin"],
  "arguments": ["0x1234567890abcdef..."]
}
```

## Response Format

Returns an array of Move values in JSON format:

```json
[
  "1000000000"
]
```

The response array contains one element per return value of the function.

---

## Function Name Format

```
{address}::{module}::{function}
```

Examples:
- `0x1::coin::balance` - Get coin balance
- `0x1::account::exists_at` - Check if account exists
- `0x1::primary_fungible_store::balance` - Get FA balance

---

## Argument Encoding

### Primitive Types

| Move Type | JSON Encoding |
|-----------|---------------|
| `u8` | number or string |
| `u16` | number or string |
| `u32` | number or string |
| `u64` | string (decimal) |
| `u128` | string (decimal) |
| `u256` | string (decimal) |
| `bool` | boolean |
| `address` | string (hex with 0x) |
| `vector<u8>` | string (hex with 0x) |

### Complex Types

| Move Type | JSON Encoding |
|-----------|---------------|
| `vector<T>` | array of encoded T |
| `String` | string (UTF-8) |
| `Option<T>` | `{"vec": []}` or `{"vec": [value]}` |

### Examples

```json
// u64 argument
"1000000"

// address argument
"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"

// vector<u8> argument (bytes)
"0x48656c6c6f"

// vector<address> argument
["0x1", "0x2", "0x3"]

// Option<u64> - None
{"vec": []}

// Option<u64> - Some(100)
{"vec": ["100"]}
```

---

## Common View Functions

### Coin Module (`0x1::coin`)

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `balance<CoinType>` | address | u64 | Get coin balance |
| `is_coin_initialized<CoinType>` | - | bool | Check if coin exists |
| `name<CoinType>` | - | String | Get coin name |
| `symbol<CoinType>` | - | String | Get coin symbol |
| `decimals<CoinType>` | - | u8 | Get coin decimals |
| `supply<CoinType>` | - | Option<u128> | Get total supply |

### Account Module (`0x1::account`)

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `exists_at` | address | bool | Check if account exists |
| `get_authentication_key` | address | vector<u8> | Get auth key |
| `get_sequence_number` | address | u64 | Get sequence number |

### Fungible Asset (`0x1::primary_fungible_store`)

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `balance` | address, Object<Metadata> | u64 | Get FA balance |
| `is_frozen` | address, Object<Metadata> | bool | Check if frozen |

### Staking (`0x1::stake`)

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `get_stake` | address | (u64, u64, u64, u64) | Get stake amounts |
| `get_validator_state` | address | u64 | Get validator state |

---

## Code Examples

### Python

```python
from typing import Any, List, Optional

def view_function(
    client: AptosClient,
    function: str,
    type_arguments: List[str] = None,
    arguments: List[Any] = None,
    ledger_version: Optional[int] = None,
) -> List[Any]:
    """Execute a view function and return results."""
    payload = {
        "function": function,
        "type_arguments": type_arguments or [],
        "arguments": arguments or [],
    }
    
    params = {}
    if ledger_version is not None:
        params["ledger_version"] = ledger_version
    
    response = requests.post(
        f"{client.base_url}/view",
        params=params,
        json=payload,
        headers={"Content-Type": "application/json"},
    )
    
    if response.ok:
        return response.json()
    else:
        error = response.json()
        raise APIError(
            error.get("message", "View function failed"),
            error.get("error_code", "vm_error"),
            response.status_code
        )


def get_apt_balance(client: AptosClient, address: str) -> int:
    """Get APT balance for an address."""
    result = view_function(
        client,
        function="0x1::coin::balance",
        type_arguments=["0x1::aptos_coin::AptosCoin"],
        arguments=[address],
    )
    return int(result[0])


def account_exists(client: AptosClient, address: str) -> bool:
    """Check if an account exists."""
    result = view_function(
        client,
        function="0x1::account::exists_at",
        arguments=[address],
    )
    return result[0]


def get_coin_info(
    client: AptosClient,
    coin_type: str
) -> dict:
    """Get coin metadata."""
    name = view_function(
        client,
        function="0x1::coin::name",
        type_arguments=[coin_type],
        arguments=[],
    )[0]
    
    symbol = view_function(
        client,
        function="0x1::coin::symbol",
        type_arguments=[coin_type],
        arguments=[],
    )[0]
    
    decimals = view_function(
        client,
        function="0x1::coin::decimals",
        type_arguments=[coin_type],
        arguments=[],
    )[0]
    
    return {
        "name": name,
        "symbol": symbol,
        "decimals": decimals,
    }


def get_multiple_balances(
    client: AptosClient,
    addresses: List[str],
    coin_type: str = "0x1::aptos_coin::AptosCoin"
) -> dict:
    """Get balances for multiple addresses."""
    balances = {}
    
    for address in addresses:
        try:
            balance = view_function(
                client,
                function="0x1::coin::balance",
                type_arguments=[coin_type],
                arguments=[address],
            )[0]
            balances[address] = int(balance)
        except APIError as e:
            if "RESOURCE_NOT_FOUND" in str(e):
                balances[address] = 0
            else:
                raise
    
    return balances


# Example: Custom contract view function
def get_user_profile(
    client: AptosClient,
    contract_address: str,
    user_address: str
) -> dict:
    """Call a custom view function on a user contract."""
    result = view_function(
        client,
        function=f"{contract_address}::user::get_profile",
        type_arguments=[],
        arguments=[user_address],
    )
    
    # Assuming the function returns (String, u64, bool)
    return {
        "username": result[0],
        "points": int(result[1]),
        "is_active": result[2],
    }
```

### TypeScript

```typescript
interface ViewRequest {
  function: string;
  type_arguments: string[];
  arguments: any[];
}

type ViewResult = any[];

async function viewFunction(
  client: AptosClient,
  func: string,
  typeArguments: string[] = [],
  args: any[] = [],
  ledgerVersion?: number
): Promise<ViewResult> {
  const payload: ViewRequest = {
    function: func,
    type_arguments: typeArguments,
    arguments: args,
  };

  const params: Record<string, string> = {};
  if (ledgerVersion !== undefined) {
    params.ledger_version = ledgerVersion.toString();
  }

  const url = new URL(`${client.baseUrl}/view`);
  Object.entries(params).forEach(([key, value]) => {
    url.searchParams.set(key, value);
  });

  const response = await fetch(url.toString(), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const error = await response.json();
    throw new APIError(
      error.message || 'View function failed',
      error.error_code || 'vm_error',
      response.status
    );
  }

  return response.json();
}

async function getAptBalance(client: AptosClient, address: string): Promise<bigint> {
  const result = await viewFunction(
    client,
    '0x1::coin::balance',
    ['0x1::aptos_coin::AptosCoin'],
    [address]
  );
  return BigInt(result[0]);
}

async function accountExists(client: AptosClient, address: string): Promise<boolean> {
  const result = await viewFunction(
    client,
    '0x1::account::exists_at',
    [],
    [address]
  );
  return result[0];
}

interface CoinInfo {
  name: string;
  symbol: string;
  decimals: number;
}

async function getCoinInfo(client: AptosClient, coinType: string): Promise<CoinInfo> {
  const [name] = await viewFunction(client, '0x1::coin::name', [coinType], []);
  const [symbol] = await viewFunction(client, '0x1::coin::symbol', [coinType], []);
  const [decimals] = await viewFunction(client, '0x1::coin::decimals', [coinType], []);

  return {
    name,
    symbol,
    decimals: Number(decimals),
  };
}

// Batch helper with concurrency control
async function batchViewCalls<T>(
  calls: Array<() => Promise<T>>,
  concurrency: number = 5
): Promise<T[]> {
  const results: T[] = [];
  
  for (let i = 0; i < calls.length; i += concurrency) {
    const batch = calls.slice(i, i + concurrency);
    const batchResults = await Promise.all(batch.map(call => call()));
    results.push(...batchResults);
  }
  
  return results;
}

// Example: Get balances for multiple addresses
async function getMultipleBalances(
  client: AptosClient,
  addresses: string[],
  coinType: string = '0x1::aptos_coin::AptosCoin'
): Promise<Map<string, bigint>> {
  const balances = new Map<string, bigint>();

  const calls = addresses.map(address => async () => {
    try {
      const balance = await viewFunction(
        client,
        '0x1::coin::balance',
        [coinType],
        [address]
      );
      return { address, balance: BigInt(balance[0]) };
    } catch (error) {
      if (error instanceof APIError && error.message.includes('RESOURCE_NOT_FOUND')) {
        return { address, balance: BigInt(0) };
      }
      throw error;
    }
  });

  const results = await batchViewCalls(calls, 10);
  
  for (const { address, balance } of results) {
    balances.set(address, balance);
  }

  return balances;
}

export { viewFunction, getAptBalance, accountExists, getCoinInfo };
```

### Rust

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct ViewRequest {
    function: String,
    type_arguments: Vec<String>,
    arguments: Vec<Value>,
}

async fn view_function(
    client: &Client,
    base_url: &str,
    function: &str,
    type_arguments: Vec<&str>,
    arguments: Vec<Value>,
    ledger_version: Option<u64>,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut url = format!("{}/view", base_url);
    
    if let Some(version) = ledger_version {
        url = format!("{}?ledger_version={}", url, version);
    }
    
    let payload = ViewRequest {
        function: function.to_string(),
        type_arguments: type_arguments.iter().map(|s| s.to_string()).collect(),
        arguments,
    };
    
    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        let error: AptosError = response.json().await?;
        Err(format!("View failed: {}", error.message).into())
    }
}

async fn get_apt_balance(
    client: &Client,
    base_url: &str,
    address: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let result = view_function(
        client,
        base_url,
        "0x1::coin::balance",
        vec!["0x1::aptos_coin::AptosCoin"],
        vec![Value::String(address.to_string())],
        None,
    ).await?;
    
    let balance_str = result[0].as_str().ok_or("Invalid balance")?;
    Ok(balance_str.parse()?)
}

async fn account_exists(
    client: &Client,
    base_url: &str,
    address: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let result = view_function(
        client,
        base_url,
        "0x1::account::exists_at",
        vec![],
        vec![Value::String(address.to_string())],
        None,
    ).await?;
    
    Ok(result[0].as_bool().unwrap_or(false))
}
```

---

## Historical Queries

Query state at a specific ledger version:

```python
# Get balance at a specific version
historical_balance = view_function(
    client,
    function="0x1::coin::balance",
    type_arguments=["0x1::aptos_coin::AptosCoin"],
    arguments=["0x1234..."],
    ledger_version=100000000
)
```

**Note**: Historical data availability depends on node pruning configuration.

---

## Error Responses

### Function Not Found

```json
{
  "message": "Function not found",
  "error_code": "function_not_found",
  "vm_error_code": null
}
```

### Invalid Arguments

```json
{
  "message": "Invalid arguments for function",
  "error_code": "invalid_input",
  "vm_error_code": null
}
```

### VM Error

```json
{
  "message": "Move abort in 0x1::coin: ECOIN_STORE_NOT_PUBLISHED(6)",
  "error_code": "vm_error",
  "vm_error_code": 393222
}
```

### Common VM Error Codes

| Code | Module | Description |
|------|--------|-------------|
| 393222 | coin | ECOIN_STORE_NOT_PUBLISHED |
| 524295 | account | EACCOUNT_DOES_NOT_EXIST |
| 65537 | error | EOUT_OF_RANGE |

---

## Best Practices

1. **Handle missing resources** - Check `exists_at` before querying account data
2. **Batch related queries** - Minimize round trips for multiple view calls
3. **Cache static data** - Coin metadata rarely changes
4. **Use historical queries carefully** - Old versions may be pruned
5. **Validate addresses** - Invalid addresses cause errors

## Related Documents

- [API Overview](01-api-overview.md) - Base URLs and common patterns
- [Accounts API](02-accounts-api.md) - Alternative resource queries
- [Transactions API](03-transactions-api.md) - Simulation for write operations
