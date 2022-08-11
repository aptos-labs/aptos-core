/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $VersionedEvent = {
    properties: {
        version: {
            type: 'U64',
            isRequired: true,
        },
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
