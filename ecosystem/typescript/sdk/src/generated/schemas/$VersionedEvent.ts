/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $VersionedEvent = {
    description: `An event from a transaction with a version`,
    properties: {
        version: {
            type: 'U64',
            isRequired: true,
        },
        guid: {
            type: 'all-of',
            contains: [{
                type: 'EventKeyWrapper',
            }],
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
            type: 'MoveType',
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
