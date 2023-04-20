/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveValue = {
    type: 'any-of',
    description: `An enum of the possible Move value types`,
    contains: [{
        type: 'number',
        format: 'uint8',
    }, {
        type: 'number',
        format: 'uint16',
    }, {
        type: 'number',
        format: 'uint32',
    }, {
        type: 'U64',
    }, {
        type: 'U128',
    }, {
        type: 'U256',
    }, {
        type: 'boolean',
    }, {
        type: 'Address',
    }, {
        type: 'array',
        contains: {
            type: 'MoveValue',
        },
    }, {
        type: 'HexEncodedBytes',
    }, {
        type: 'MoveStructValue',
    }, {
        type: 'string',
    }],
} as const;
