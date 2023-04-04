/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetChange = {
    type: 'one-of',
    description: `A final state change of a transaction on a resource or module`,
    contains: [{
        type: 'WriteSetChange_DeleteModule',
    }, {
        type: 'WriteSetChange_DeleteResource',
    }, {
        type: 'WriteSetChange_DeleteTableItem',
    }, {
        type: 'WriteSetChange_WriteModule',
    }, {
        type: 'WriteSetChange_WriteResource',
    }, {
        type: 'WriteSetChange_WriteTableItem',
    }],
} as const;
