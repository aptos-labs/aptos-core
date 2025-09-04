/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export { VelorFaucetClient } from './VelorFaucetClient';

export { ApiError } from './core/ApiError';
export { BaseHttpRequest } from './core/BaseHttpRequest';
export { CancelablePromise, CancelError } from './core/CancelablePromise';
export { OpenAPI } from './core/OpenAPI';
export type { OpenAPIConfig } from './core/OpenAPI';

export type { VelorTapError } from './models/VelorTapError';
export { VelorTapErrorCode } from './models/VelorTapErrorCode';
export type { FundRequest } from './models/FundRequest';
export type { FundResponse } from './models/FundResponse';
export type { RejectionReason } from './models/RejectionReason';
export { RejectionReasonCode } from './models/RejectionReasonCode';

export { $VelorTapError } from './schemas/$VelorTapError';
export { $VelorTapErrorCode } from './schemas/$VelorTapErrorCode';
export { $FundRequest } from './schemas/$FundRequest';
export { $FundResponse } from './schemas/$FundResponse';
export { $RejectionReason } from './schemas/$RejectionReason';
export { $RejectionReasonCode } from './schemas/$RejectionReasonCode';

export { CaptchaService } from './services/CaptchaService';
export { FundService } from './services/FundService';
export { GeneralService } from './services/GeneralService';
