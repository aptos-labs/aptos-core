/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $NodeAddress = {
    properties: {
        url: {
            type: 'string',
            description: `Target URL. This should include a scheme (e.g. http://). If there is
            no scheme, we will prepend http://.`,
            isRequired: true,
            format: 'url',
        },
        metrics_port: {
            type: 'number',
            description: `Metrics port.`,
            format: 'uint16',
        },
        api_port: {
            type: 'number',
            description: `API port.`,
            format: 'uint16',
        },
        noise_port: {
            type: 'number',
            description: `Validator communication port.`,
            format: 'uint16',
        },
    },
} as const;
