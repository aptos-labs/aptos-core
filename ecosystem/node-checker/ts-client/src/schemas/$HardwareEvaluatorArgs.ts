/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $HardwareEvaluatorArgs = {
    properties: {
        min_cpu_cores: {
            type: 'number',
            description: `The minimum number of physical CPU cores the machine must have.`,
            isRequired: true,
            format: 'uint64',
        },
        min_ram_gb: {
            type: 'number',
            description: `The minimum amount of RAM in GB (not GiB) the machine must have.`,
            isRequired: true,
            format: 'uint64',
        },
    },
} as const;
