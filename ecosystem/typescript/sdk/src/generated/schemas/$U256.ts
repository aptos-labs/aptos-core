/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $U256 = {
    type: 'string',
    description: `A string containing a 256-bit unsigned integer.

    We represent u256 values as a string to ensure compatibility with languages such
    as JavaScript that do not parse u256s in JSON natively.
    `,
    format: 'uint256',
} as const;
