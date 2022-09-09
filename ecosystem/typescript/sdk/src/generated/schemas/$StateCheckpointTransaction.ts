/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $StateCheckpointTransaction = {
    description: `A state checkpoint transaction`,
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
        timestamp: {
            type: 'U64',
            isRequired: true,
        },
    },
} as const;
