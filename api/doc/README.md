# Aptos Node API v1

## Overview
Aptos Node API v1 provides a RESTful interface for interacting with Aptos blockchain nodes. The API enables users to retrieve blockchain information, submit transactions, and query account states.

## Key Features
- Account state and resources querying
- Transaction submission and monitoring
- Block and event information retrieval
- Validator data access
- Smart contract interaction

## Getting Started
1. Ensure you have an Aptos node running
2. API is available by default on port 8080
3. Use any REST client to send requests

## Authentication
The API does not require authentication for public endpoints. Some administrative endpoints may require additional authorization.

## Data Formats
- All requests and responses use JSON format
- Transactions must be signed using Ed25519
- Timestamps are represented in UTC ISO 8601 format

## Limitations
- Rate limiting: 100 requests per minute by default
- Maximum request size: 2MB
- Connection timeout: 30 seconds

## Versioning
The API follows semantic versioning. Current v1 version ensures backward compatibility within the major version.

## API Documentation
Complete OpenAPI specification is available at `/api/v1/spec`

## Support
- [GitHub Issues](https://github.com/aptos-labs/aptos-core/issues)
- [Discord](https://discord.gg/aptosnetwork)
- [Aptos Documentation](https://aptos.dev)

## Contributing
We welcome community contributions! Please review our [contribution guidelines](../CONTRIBUTING.md) before submitting a pull request.
