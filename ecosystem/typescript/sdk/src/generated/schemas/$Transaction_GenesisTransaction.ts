/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction_GenesisTransaction = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'GenesisTransaction',
    }],
} as const;
