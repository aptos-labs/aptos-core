/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $UserTransaction = {
    description: `A transaction submitted by a user to change the state of the blockchain`,
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
        sender: {
            type: 'all-of',
            contains: [{
                type: 'Address',
            }],
            isRequired: true,
        },
        sequence_number: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        max_gas_amount: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        gas_unit_price: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        expiration_timestamp_secs: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        payload: {
            type: 'all-of',
            contains: [{
                type: 'TransactionPayload',
            }],
            isRequired: true,
        },
        signature: {
            type: 'all-of',
            contains: [{
                type: 'TransactionSignature',
            }],
        },
        events: {
            type: 'array',
            contains: {
                type: 'Event',
            },
            isRequired: true,
        },
        timestamp: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
    },
} as const;
