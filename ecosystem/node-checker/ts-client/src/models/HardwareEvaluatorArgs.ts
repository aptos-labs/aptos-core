/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type HardwareEvaluatorArgs = {
    /**
     * The minimum number of physical CPU cores the machine must have.
     */
    min_cpu_cores: number;
    /**
     * The minimum amount of RAM in GB (not GiB) the machine must have.
     */
    min_ram_gb: number;
};

