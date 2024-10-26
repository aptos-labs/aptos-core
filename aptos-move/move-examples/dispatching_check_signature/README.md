# Dispatching Check Signature

This is a simple example of how to use the dispatching check signature. This project aims to implement functionality similar to EIP-1271, allowing smart contracts to verify signatures.

## Main Features

The project implements an extensible signature verification system that allows different modules to register their own signature verification logic. The main features include:

1. Registering dispatchable signature verification functions
2. Storing and retrieving data associated with module addresses
3. Verifying signatures for given module addresses

## Contract Functionalities

### storage.move

The `storage` module is responsible for managing the dispatcher's storage. Its main functionalities include:

- Initializing the storage structure
- Storing and retrieving data associated with module addresses
- Managing metadata for the dispatcher
- Providing access control functionality

### check_signature.move

The `check_signature` module implements the main signature verification logic. Its main functionalities include:

- Checking signatures for given module addresses
- Registering dispatchable signature verification functions
- Dispatching signature verification requests to the appropriate modules

## Usage

To use this system, module developers need to:

1. Call the `register_dispatchable` function to register their signature verification logic
2. Implement a signature verification function that conforms to the specified interface

Users can verify signatures for specific module addresses by calling the `check_signature` function.
