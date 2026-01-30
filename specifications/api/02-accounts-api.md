# Accounts API Specification

> **Version:** 1.0.0  
> **Last Updated:** January 28, 2026

## Overview

The Accounts API provides endpoints for querying account information, resources, modules, and balances on the Aptos blockchain.

## Endpoints Summary

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/accounts/{address}` | Get account info |
| GET | `/accounts/{address}/resources` | List all resources |
| GET | `/accounts/{address}/resource/{type}` | Get specific resource |
| GET | `/accounts/{address}/modules` | List all modules |
| GET | `/accounts/{address}/module/{name}` | Get specific module |
| GET | `/accounts/{address}/balance/{asset_type}` | Get asset balance |
| GET | `/accounts/{address}/transactions` | Get account transactions |

## Get Account

Retrieves the authentication key and sequence number for an account.

```
GET /v1/accounts/{address}
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address (with or without `0x` prefix) |
| `ledger_version` | query | u64 | No | Ledger version for historical query |

### Response

```json
{
  "sequence_number": "42",
  "authentication_key": "0x1234567890abcdef..."
}
```

### Example

```bash
curl "https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1"
```

### Code Examples

```python
def get_account(client: AptosClient, address: str) -> dict:
    """Get account sequence number and authentication key."""
    response = client.get(f"/accounts/{address}")
    return {
        "sequence_number": int(response["sequence_number"]),
        "authentication_key": response["authentication_key"],
    }
```

```typescript
async function getAccount(client: AptosClient, address: string): Promise<{
  sequenceNumber: bigint;
  authenticationKey: string;
}> {
  const response = await client.get(`/accounts/${address}`);
  return {
    sequenceNumber: BigInt(response.sequence_number),
    authenticationKey: response.authentication_key,
  };
}
```

---

## Get Account Resources

Retrieves all resources stored on an account.

```
GET /v1/accounts/{address}/resources
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `ledger_version` | query | u64 | No | Ledger version |
| `start` | query | string | No | Cursor for pagination |
| `limit` | query | u16 | No | Max items to return (default: 25) |

### Response

```json
[
  {
    "type": "0x1::account::Account",
    "data": {
      "authentication_key": "0x...",
      "coin_register_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0x...",
            "creation_num": "0"
          }
        }
      },
      "guid_creation_num": "4",
      "key_rotation_events": {...},
      "rotation_capability_offer": {...},
      "sequence_number": "42",
      "signer_capability_offer": {...}
    }
  },
  {
    "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
    "data": {
      "coin": {
        "value": "100000000"
      },
      "deposit_events": {...},
      "frozen": false,
      "withdraw_events": {...}
    }
  }
]
```

### Common Resource Types

| Type | Description |
|------|-------------|
| `0x1::account::Account` | Core account data |
| `0x1::coin::CoinStore<T>` | Token balance |
| `0x1::staking_contract::Store` | Staking information |
| `0x3::token::TokenStore` | NFT token store (v1) |
| `0x4::collection::Collection` | NFT collection (v2) |

### Code Examples

```python
def get_all_resources(
    client: AptosClient,
    address: str
) -> list:
    """Get all resources with pagination."""
    resources = []
    start = None
    
    while True:
        params = {"limit": 100}
        if start:
            params["start"] = start
        
        response = client.get(f"/accounts/{address}/resources", params=params)
        resources.extend(response["data"])
        
        # Check for more pages
        cursor = response["headers"].get("X-APTOS-CURSOR")
        if not cursor:
            break
        start = cursor
    
    return resources

def get_apt_balance(client: AptosClient, address: str) -> int:
    """Get APT balance for an account."""
    resource = client.get(
        f"/accounts/{address}/resource/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
    )
    return int(resource["data"]["coin"]["value"])
```

---

## Get Specific Resource

Retrieves a specific resource by type.

```
GET /v1/accounts/{address}/resource/{resource_type}
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `resource_type` | path | string | Yes | Fully qualified type (URL encoded) |
| `ledger_version` | query | u64 | No | Ledger version |

### Resource Type Format

```
{address}::{module}::{struct}<{type_args}>
```

Examples:
- `0x1::account::Account`
- `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>`
- `0x1::coin::CoinStore<0x123::my_coin::MyCoin>`

**Note**: The `<` and `>` characters must be URL-encoded as `%3C` and `%3E`.

### Example

```bash
# Get APT balance
curl "https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::coin::CoinStore%3C0x1::aptos_coin::AptosCoin%3E"
```

### Code Examples

```python
from urllib.parse import quote

def get_resource(
    client: AptosClient,
    address: str,
    resource_type: str
) -> dict:
    """Get a specific resource by type."""
    # URL encode the resource type
    encoded_type = quote(resource_type, safe="")
    response = client.get(f"/accounts/{address}/resource/{encoded_type}")
    return response["data"]

# Example: Get coin balance
def get_coin_balance(
    client: AptosClient,
    address: str,
    coin_type: str = "0x1::aptos_coin::AptosCoin"
) -> int:
    """Get balance of a specific coin type."""
    resource_type = f"0x1::coin::CoinStore<{coin_type}>"
    try:
        resource = get_resource(client, address, resource_type)
        return int(resource["data"]["coin"]["value"])
    except APIError as e:
        if e.error_code == "resource_not_found":
            return 0
        raise
```

```typescript
function encodeResourceType(resourceType: string): string {
  return encodeURIComponent(resourceType);
}

async function getResource<T>(
  client: AptosClient,
  address: string,
  resourceType: string
): Promise<T> {
  const encoded = encodeResourceType(resourceType);
  const response = await client.get(`/accounts/${address}/resource/${encoded}`);
  return response.data as T;
}

// Example: Get coin balance
interface CoinStore {
  coin: { value: string };
  frozen: boolean;
}

async function getCoinBalance(
  client: AptosClient,
  address: string,
  coinType: string = '0x1::aptos_coin::AptosCoin'
): Promise<bigint> {
  const resourceType = `0x1::coin::CoinStore<${coinType}>`;
  try {
    const resource = await getResource<CoinStore>(client, address, resourceType);
    return BigInt(resource.coin.value);
  } catch (error) {
    if (error instanceof APIError && error.errorCode === 'resource_not_found') {
      return BigInt(0);
    }
    throw error;
  }
}
```

---

## Get Account Modules

Retrieves all modules published to an account.

```
GET /v1/accounts/{address}/modules
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `ledger_version` | query | u64 | No | Ledger version |
| `start` | query | string | No | Cursor for pagination |
| `limit` | query | u16 | No | Max items to return |

### Response

```json
[
  {
    "bytecode": "0xa11ceb0b...",
    "abi": {
      "address": "0x1",
      "name": "account",
      "friends": [],
      "exposed_functions": [
        {
          "name": "create_account",
          "visibility": "public",
          "is_entry": true,
          "is_view": false,
          "generic_type_params": [],
          "params": ["address"],
          "return": []
        }
      ],
      "structs": [...]
    }
  }
]
```

---

## Get Specific Module

Retrieves a specific module by name.

```
GET /v1/accounts/{address}/module/{module_name}
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `module_name` | path | string | Yes | Module name |
| `ledger_version` | query | u64 | No | Ledger version |

### Example

```bash
curl "https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/module/account"
```

---

## Get Account Balance

Retrieves the balance of a fungible asset.

```
GET /v1/accounts/{address}/balance/{asset_type}
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `asset_type` | path | string | Yes | Asset type (URL encoded) |
| `ledger_version` | query | u64 | No | Ledger version |

### Response

```json
{
  "balance": "100000000"
}
```

### Asset Type Format

For native APT:
```
0x1::aptos_coin::AptosCoin
```

For fungible assets (FA):
```
{metadata_address}
```

---

## Get Account Transactions

Retrieves transactions sent from an account.

```
GET /v1/accounts/{address}/transactions
```

### Parameters

| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| `address` | path | Address | Yes | Account address |
| `start` | query | u64 | No | Starting sequence number |
| `limit` | query | u16 | No | Max transactions to return |

### Response

Returns array of transaction objects (see Transactions API).

### Code Examples

```python
def get_account_transactions(
    client: AptosClient,
    address: str,
    start_seq: int = None,
    limit: int = 25
) -> list:
    """Get transactions sent from an account."""
    params = {"limit": limit}
    if start_seq is not None:
        params["start"] = start_seq
    
    response = client.get(f"/accounts/{address}/transactions", params=params)
    return response["data"]
```

---

## Error Responses

### Account Not Found (404)

```json
{
  "message": "Account not found by Address(0x...) and target ledger version 12345",
  "error_code": "account_not_found",
  "vm_error_code": null
}
```

### Resource Not Found (404)

```json
{
  "message": "Resource not found by Address(0x...) and target ledger version 12345",
  "error_code": "resource_not_found",
  "vm_error_code": null
}
```

### Version Pruned (410)

```json
{
  "message": "Ledger version 1000 is older than oldest ledger version 5000",
  "error_code": "version_pruned",
  "vm_error_code": null
}
```

## Related Documents

- [API Overview](01-api-overview.md) - Base URLs and common patterns
- [Transactions API](03-transactions-api.md) - Transaction operations
- [View Functions](05-view-functions.md) - Querying contract state
