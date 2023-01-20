/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ConfigurationDescriptor = {
    properties: {
        id: {
            type: 'string',
            description: `Configuration ID, for example devnet_fullnode.`,
            isRequired: true,
        },
        pretty_name: {
            type: 'string',
            description: `Configuration pretty name, for example "Devnet Fullnode Checker".`,
            isRequired: true,
        },
    },
} as const;
