---
title: "Indexer API"
---

# Indexer API

This section contains documentation for the Aptos Indexer API, the API built upon the standard set of processors provided in the [aptos-labs/aptos-indexer-processors](https://github.com/aptos-labs/aptos-indexer-processors) repo.

## Usage Guide

### Address Format

When making a query where one of the query params is an account address (e.g. owner), make sure the address starts with a prefix of `0x` followed by 64 hex characters. For example: `0xaa921481e07b82a26dbd5d3bc472b9ad82d3e5bfd248bacac160eac51687c2ff`.

### TypeScript Client

The Aptos TypeScript SDK provides an IndexerClient for making queries to the Aptos Indexer API. Learn more [here](/sdks/ts-sdk/typescript-sdk-indexer-client-class).
