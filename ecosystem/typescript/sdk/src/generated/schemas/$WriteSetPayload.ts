/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $WriteSetPayload = {
    description: `A writeset payload, used only for genesis`,
    properties: {
        write_set: {
            type: 'WriteSet',
            isRequired: true,
        },
    },
} as const;
