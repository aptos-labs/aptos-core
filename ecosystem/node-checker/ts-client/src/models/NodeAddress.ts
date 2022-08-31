/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type NodeAddress = {
    /**
     * Target URL. This should include a scheme (e.g. http://). If there is
     * no scheme, we will prepend http://.
     */
    url: string;
    /**
     * Metrics port.
     */
    metrics_port?: number;
    /**
     * API port.
     */
    api_port?: number;
    /**
     * Validator communication port.
     */
    noise_port?: number;
};

