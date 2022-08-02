/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $BlockMetadataTransaction = {
    properties: {
        version: {
            type: 'U64',
            isRequired: true,
        },
        hash: {
            type: 'HashValue',
            isRequired: true,
        },
        state_root_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        event_root_hash: {
            type: 'HashValue',
            isRequired: true,
        },
        gas_used: {
            type: 'U64',
            isRequired: true,
        },
        success: {
            type: 'boolean',
            isRequired: true,
        },
        vm_status: {
            type: 'string',
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
        previous_block_votes: {
            type: 'array',
            contains: {
                type: 'boolean',
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
