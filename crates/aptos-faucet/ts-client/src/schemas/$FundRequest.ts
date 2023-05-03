/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $FundRequest = {
    properties: {
        amount: {
            type: 'number',
            description: `If not set, the default is the preconfigured max funding amount. If set,
            we will use this amount instead assuming it is < than the maximum,
            otherwise we'll just use the maximum.`,
            format: 'uint64',
        },
        auth_key: {
            type: 'string',
            description: `Either this or \`address\` / \`pub_key\` must be provided.`,
        },
        address: {
            type: 'string',
            description: `Either this or \`auth_key\` / \`pub_key\` must be provided.`,
        },
        pub_key: {
            type: 'string',
            description: `Either this or \`auth_key\` / \`address\` must be provided.`,
        },
    },
} as const;
