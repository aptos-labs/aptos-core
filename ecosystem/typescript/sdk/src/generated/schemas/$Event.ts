/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Event = {
    properties: {
        key: {
            type: 'EventKey',
            isRequired: true,
        },
        sequence_number: {
            type: 'U64',
            isRequired: true,
        },
        type: {
            type: 'MoveType',
            isRequired: true,
        },
        data: {
            properties: {
            },
            isRequired: true,
        },
    },
} as const;
