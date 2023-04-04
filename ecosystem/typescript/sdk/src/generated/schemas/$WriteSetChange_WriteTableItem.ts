/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_WriteTableItem = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'WriteTableItem',
    }],
} as const;
