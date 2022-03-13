# On-Chain Data and Transactions

Diem transactions mutate and create state (or resources) within the set of [on-chain modules](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/framework/core/sources), primarily the [Diem Account](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/doc/DiemAccount.md). The transaction format is defined in the [Move Adapter Specification](https://github.com/aptos-labs/aptos-core/blob/main/specifications/move_adapter/README.md). Most participants of the Diem Payment Network (DPN) will submit SignedTransactions containing a [script function](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/script_documentation/script_documentation.md). Before release 1.2, clients used scripts. These can be accessed in [compiled form](https://github.com/aptos-labs/aptos-core/tree/release-1.1/language/stdlib/compiled/transaction_scripts) and in their [original form](https://github.com/aptos-labs/aptos-core/tree/release-1.1/language/stdlib/transaction_scripts). The DPN MainNet only allows script functions and this set of pre-registerd scripts to be submitted. Due to the evolving nature of Move and the Move compiler, compiling existing scripts may not result in the form stored in the directory stored above. Hence, it is recommended to use script functions where available or otherwise the compiled scripts.

## Peer to Peer Payments and Transaction Metadata

Most transactions will use the [peer_to_peer_with_metadata script function](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/script_documentation/script_documentation.md#0x1_PaymentScripts_peer_to_peer_with_metadata). This single transaction represents all current transfers between two participants and distinguishes the types of transfers via the embedded metadata.

The metadata is represented by the following `Rust` enum encoded in [Binary Canonical Serialization (BCS)](https://github.com/diem/bcs):

```
enum Metadata {
  Undefined,
  GeneralMetadata(GeneralMetadata),
  UnstructuredByteMetadata(Option<Vec<u8>>),
  RefundMetadata(RefundMetadata),
}
```

Note: This is the canonical list and should be referred to in future DIPs so that authors need not reproduce the list in future DIPs.

## Payments Using GeneralMetadata

```
enum GeneralMetadata {
   GeneralMetadataV0(GeneralMetadataV0),
}

struct GeneralMetadataV0 {
   to_subaddress: Option<Vec<u8>>,
   from_subaddress: Option<Vec<u8>>,
   referenced_event: Option<u64>, // Deprecated
}
```

GeneralMetadata leverages the notion of subaddresses to indicate a source and destination and are stored in the fields `from_subaddress` and `to_subaddress`, respectively.

Subaddresses have the following properties:
* 8-bytes
* Subaddresses should be unique
* The address represented by 8 zero bytes (or None/Null within the GeneralMetadataV0) is reserved to denote the root (VASP owned) account

Lifetime of subaddresses:
* There are no explicit lifetimes of subaddresses
* The same `from_address` may receive multiple payments from distinct `to_subaddress`
* A `from_subaddress` may be the recipient or a `to_subaddress` in an ensuing transaction
* `to_subaddress` should be generated fresh each time upon user request
* `from_subaddress` should be unique for each transaction

Subaddresses should be used with great care to not accidentally leak personally identifiable information (PII). However, implementors must be mindful of the permissive nature of subaddresses as outlined in this specification.

## Dual Attestation Credentials

Diem defines a [DualAttestation::Credential](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/core/sources/DualAttestation.move) resource to support off-chain protocols. This resource contains the `human_name`, `base_url`, and `compliance_public_key` for a VASP. The `base_url` specifies where the VASP hosts its off-chain API and the `compliance_public_key` is used to verify signed transaction metadata and establish authentication in off-chain communication. The values can be set and updated via the [rotate_dual_attestation_info](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/transaction_scripts/rotate_dual_attestation_info.move) script.
