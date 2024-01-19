/// Common types.
module aptos_framework::types {
    /// Information about a validator that participates consensus.
    struct ValidatorConsensusInfo has copy, drop, store {
        addr: address,
        pk_bytes: vector<u8>,
        voting_power: u64,
    }

    /// Create a `ValidatorConsensusInfo` object.
    public fun default_validator_consensus_info(): ValidatorConsensusInfo {
        ValidatorConsensusInfo {
            addr: @vm,
            pk_bytes: vector[],
            voting_power: 0,
        }
    }

    /// Create a `ValidatorConsensusInfo` object.
    public fun new_validator_consensus_info(addr: address, pk_bytes: vector<u8>, voting_power: u64): ValidatorConsensusInfo {
        ValidatorConsensusInfo {
            addr,
            pk_bytes,
            voting_power,
        }
    }
}
