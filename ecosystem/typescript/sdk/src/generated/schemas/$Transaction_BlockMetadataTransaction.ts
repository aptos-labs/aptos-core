/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction_BlockMetadataTransaction = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'BlockMetadataTransaction',
    }],
} as const;
