/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $BlockMetadataTransaction = {
    description: `A block metadata transaction

    This signifies the beginning of a block, and contains information
    about the specific block`,
    properties: {
        version: {
            type: 'U64',
            isRequired: true,
        },
        hash: {
            type: 'HashValue',
            isRequired: true,
        },
        state_change_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        event_root_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        state_checkpoint_hash: {
            type: 'HashValue',
        },
        gas_used: {
            type: 'U64',
            isRequired: true,
        },
        success: {
            type: 'boolean',
            description: `Whether the transaction was successful`,
            isRequired: true,
        },
        vm_status: {
            type: 'string',
            description: `The VM status of the transaction, can tell useful information in a failure`,
            isRequired: true,
        },
        accumulator_root_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        changes: {
            type: 'array',
            contains: {
                type: 'WriteSetChange',
            },
            isRequired: true,
        },
        id: {
            type: 'HashValue',
            isRequired: true,
        },
        epoch: {
            type: 'U64',
            isRequired: true,
        },
        round: {
            type: 'U64',
            isRequired: true,
        },
        events: {
            type: 'array',
            contains: {
                type: 'Event',
            },
            isRequired: true,
        },
        previous_block_votes_bitvec: {
            type: 'array',
            contains: {
                type: 'number',
                format: 'uint8',
            },
            isRequired: true,
        },
        proposer: {
            type: 'Address',
            isRequired: true,
        },
        failed_proposer_indices: {
            type: 'array',
            contains: {
                type: 'number',
                format: 'uint32',
            },
            isRequired: true,
        },
        timestamp: {
            type: 'U64',
            isRequired: true,
        },
    },
} as const;
