/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Event = {
    description: `An event from a transaction`,
    properties: {
        guid: {
            type: 'EventGuid',
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
            description: `The JSON representation of the event`,
            properties: {
            },
            isRequired: true,
        },
    },
} as const;
