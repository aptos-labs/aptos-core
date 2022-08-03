/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction_StateCheckpointTransaction = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'StateCheckpointTransaction',
    }],
} as const;
