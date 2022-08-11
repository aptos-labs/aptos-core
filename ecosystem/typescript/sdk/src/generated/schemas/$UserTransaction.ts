/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $UserTransaction = {
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
        sender: {
            type: 'Address',
            isRequired: true,
        },
        sequence_number: {
            type: 'U64',
            isRequired: true,
        },
        max_gas_amount: {
            type: 'U64',
            isRequired: true,
        },
        gas_unit_price: {
            type: 'U64',
            isRequired: true,
        },
        expiration_timestamp_secs: {
            type: 'U64',
            isRequired: true,
        },
        payload: {
            type: 'TransactionPayload',
            isRequired: true,
        },
        signature: {
            type: 'TransactionSignature',
        },
        events: {
            type: 'array',
            contains: {
                type: 'Event',
            },
            isRequired: true,
        },
        timestamp: {
            type: 'U64',
            isRequired: true,
        },
    },
} as const;
