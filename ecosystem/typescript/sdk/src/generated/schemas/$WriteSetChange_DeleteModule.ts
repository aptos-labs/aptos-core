/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange_DeleteModule = {
    type: 'all-of',
    contains: [{
        properties: {
            type: {
                type: 'string',
                isRequired: true,
            },
        },
    }, {
        type: 'DeleteModule',
    }],
} as const;
