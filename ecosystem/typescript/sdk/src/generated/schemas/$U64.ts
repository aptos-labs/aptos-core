/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $U64 = {
    type: 'string',
    description: `A string containing a 64-bit unsigned integer.

    We represent u64 values as a string to ensure compatibility with languages such
    as JavaScript that do not parse u64s in JSON natively.
    `,
    format: 'uint64',
} as const;
