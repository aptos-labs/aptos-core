/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $RejectionReason = {
    properties: {
        reason: {
            type: 'string',
            isRequired: true,
        },
        code: {
            type: 'RejectionReasonCode',
            isRequired: true,
        },
    },
} as const;
