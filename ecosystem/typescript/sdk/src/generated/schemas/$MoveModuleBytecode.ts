/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $MoveModuleBytecode = {
    description: `Move module bytecode along with it's ABI`,
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
                type: 'MoveModule',
            }],
        },
    },
} as const;
