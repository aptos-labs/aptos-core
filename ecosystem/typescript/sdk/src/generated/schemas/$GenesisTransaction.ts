/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $GenesisTransaction = {
    description: `The genesis transaction

    This only occurs at the genesis transaction (version 0)`,
    properties: {
        version: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        hash: {
            type: 'all-of',
            contains: [{
                type: 'HashValue',
            }],
            isRequired: true,
        },
        state_change_hash: {
            type: 'all-of',
            contains: [{
                type: 'HashValue',
            }],
            isRequired: true,
        },
        event_root_hash: {
            type: 'all-of',
            contains: [{
                type: 'HashValue',
            }],
            isRequired: true,
        },
        state_checkpoint_hash: {
            type: 'all-of',
            contains: [{
                type: 'HashValue',
            }],
        },
        gas_used: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
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
            type: 'all-of',
            contains: [{
                type: 'HashValue',
            }],
            isRequired: true,
        },
        changes: {
            type: 'array',
            contains: {
                type: 'WriteSetChange',
            },
            isRequired: true,
        },
        payload: {
            type: 'all-of',
            contains: [{
                type: 'GenesisPayload',
            }],
            isRequired: true,
        },
        events: {
            type: 'array',
            contains: {
                type: 'Event',
            },
            isRequired: true,
        },
    },
} as const;
