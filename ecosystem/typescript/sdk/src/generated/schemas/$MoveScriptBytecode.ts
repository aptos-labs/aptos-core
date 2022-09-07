/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveScriptBytecode = {
    description: `Move script bytecode`,
    properties: {
        bytecode: {
            type: 'all-of',
            contains: [{
                type: 'HexEncodedBytes',
            }],
            isRequired: true,
        },
        abi: {
            type: 'all-of',
            contains: [{
                type: 'MoveFunction',
            }],
        },
    },
} as const;
