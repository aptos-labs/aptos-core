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

    /// Get `ValidatorConsensusInfo.addr`.
    public fun addr_from_validator_consensus_info(vci: &ValidatorConsensusInfo): address {
        vci.addr
    }

    /// Get `ValidatorConsensusInfo.pk_bytes`.
    public fun pk_bytes_from_validator_consensus_info(vci: &ValidatorConsensusInfo): vector<u8> {
        vci.pk_bytes
    }

    /// Get `ValidatorConsensusInfo.voting_power`.
    public fun voting_power_from_validator_consensus_info(vci: &ValidatorConsensusInfo): u64 {
        vci.voting_power
    }
}
