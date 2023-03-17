/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type FundRequest = {
    /**
     * If not set, the default is the preconfigured max funding amount. If set,
     * we will use this amount instead assuming it is < than the maximum,
     * otherwise we'll just use the maximum.
     */
    amount?: number;
    /**
     * Either this or `address` / `pub_key` must be provided.
     */
    auth_key?: string;
    /**
     * Either this or `auth_key` / `pub_key` must be provided.
     */
    address?: string;
    /**
     * Either this or `auth_key` / `address` must be provided.
     */
    pub_key?: string;
};

