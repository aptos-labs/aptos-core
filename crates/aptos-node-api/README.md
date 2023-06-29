# Aptos Node API

This directory contains code for the API that runs on the Aptos node.

This is a brief explanation of the different directories here:
- [context/](context/): The context allows top level API code to access inner components of the node, such as storage, mempool, etc.
- [test-context](test-context/): This is a version of the context that we use for unit / integration tests.
- [entrypoint/](entrypoint/): The entrypoint defines the runtime that we hook into the node at startup to run the node API. This is where specific APIs (e.g. `v1/`, `v2/`) can attach routes and endpoint handlers.
- [v1/](v1/): The v1 API is an OpenAPI based API using Poem.

