/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $CheckResult = {
    properties: {
        checker_name: {
            type: 'string',
            description: `Name of the Checker that created the result.`,
            isRequired: true,
        },
        headline: {
            type: 'string',
            description: `Headline of the result, e.g. "Healthy!" or "Metrics missing!".`,
            isRequired: true,
        },
        score: {
            type: 'number',
            description: `Score out of 100.`,
            isRequired: true,
            format: 'uint8',
        },
        explanation: {
            type: 'string',
            description: `Explanation of the result.`,
            isRequired: true,
        },
        links: {
            type: 'array',
            contains: {
                type: 'string',
            },
            isRequired: true,
        },
    },
} as const;
