/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_DeleteTableItem = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'DeleteTableItem',
    }],
} as const;
