/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Address = {
    type: 'string',
    description: `A hex encoded 32 byte Aptos account address.

    This is represented in a string as a 64 character hex string, sometimes
    shortened by stripping leading 0s, and adding a 0x.

    For example, address 0x0000000000000000000000000000000000000000000000000000000000000001 is represented as 0x1.
    `,
    format: 'hex',
} as const;
