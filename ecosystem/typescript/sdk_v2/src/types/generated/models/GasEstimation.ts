/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * Struct holding the outputs of the estimate gas API
 */
export type GasEstimation = {
    /**
     * The deprioritized estimate for the gas unit price
     */
    deprioritized_gas_estimate?: number;
    /**
     * The current estimate for the gas unit price
     */
    gas_estimate: number;
    /**
     * The prioritized estimate for the gas unit price
     */
    prioritized_gas_estimate?: number;
};

