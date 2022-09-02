/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $VersionedEvent = {
    description: `An event from a transaction with a version`,
    properties: {
        version: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        key: {
            type: 'EventKey',
            isRequired: true,
        },
        sequence_number: {
            type: 'all-of',
            contains: [{
                type: 'U64',
            }],
            isRequired: true,
        },
        type: {
            type: 'all-of',
            contains: [{
                type: 'MoveType',
            }],
            isRequired: true,
        },
        data: {
            description: `The JSON representation of the event`,
            properties: {
            },
            isRequired: true,
        },
    },
} as const;
