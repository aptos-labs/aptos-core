/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $IndexResponse = {
    description: `The struct holding all data returned to the client by the
    index endpoint (i.e., GET "/").  Only for responding in JSON`,
    properties: {
        chain_id: {
            type: 'number',
            description: `Chain ID of the current chain`,
            isRequired: true,
            format: 'uint8',
        },
        epoch: {
            type: 'U64',
            isRequired: true,
        },
        ledger_version: {
            type: 'U64',
            isRequired: true,
        },
        oldest_ledger_version: {
            type: 'U64',
            isRequired: true,
        },
        ledger_timestamp: {
            type: 'U64',
            isRequired: true,
        },
        node_role: {
            type: 'RoleType',
            isRequired: true,
        },
        oldest_block_height: {
            type: 'U64',
            isRequired: true,
        },
        block_height: {
            type: 'U64',
            isRequired: true,
        },
        git_hash: {
            type: 'string',
            description: `Git hash of the build of the API endpoint.  Can be used to determine the exact
            software version used by the API endpoint.`,
        },
    },
} as const;
