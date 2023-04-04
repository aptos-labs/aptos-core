/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveModuleBytecode = {
    description: `Move module bytecode along with it's ABI`,
    properties: {
        bytecode: {
            type: 'HexEncodedBytes',
            isRequired: true,
        },
        abi: {
            type: 'MoveModule',
        },
    },
} as const;
