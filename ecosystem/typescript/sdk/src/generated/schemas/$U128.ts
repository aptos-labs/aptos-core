/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $U128 = {
    type: 'string',
    description: `A string containing a 128-bit unsigned integer.

    We represent u128 values as a string to ensure compatibility with languages such
    as JavaScript that do not parse u128s in JSON natively.
    `,
    format: 'uint128',
} as const;
