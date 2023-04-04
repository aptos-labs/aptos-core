/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveScriptBytecode = {
    description: `Move script bytecode`,
    properties: {
        bytecode: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
        abi: {
            type: 'MoveFunction',
        },
    },
} as const;
