/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction_UserTransaction = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'UserTransaction',
    }],
} as const;
