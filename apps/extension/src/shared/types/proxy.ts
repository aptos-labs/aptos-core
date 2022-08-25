// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { DappError } from 'core/types/errors';

export interface ProxiedRequest {
  args: any[],
  id: number,
  method: string,
  type: 'request',
}

export interface ProxiedResponse {
  error?: DappError,
  id: number,
  result?: any,
  type: 'response',
}

export function makeProxiedRequest(id: number, method: string, args: any[]): ProxiedRequest {
  return {
    args,
    id,
    method,
    type: 'request',
  };
}

export function makeProxiedResponse(id: number, resultOrError?: any | DappError): ProxiedResponse {
  const isError = resultOrError instanceof DappError;
  return {
    error: isError ? resultOrError : undefined,
    id,
    result: isError ? undefined : resultOrError,
    type: 'response',
  };
}

/**
 * Check if an object is a ProxiedRequest
 */
export function isProxiedRequest(data: ProxiedRequest): data is ProxiedRequest {
  return data.type === 'request'
    && data.id !== undefined
    && data.id >= 0
    && data.method !== undefined
    && data.args !== undefined;
}

/**
 * Check if an object is a ProxiedResponse
 */
export function isProxiedResponse(data: ProxiedResponse): data is ProxiedResponse {
  return data.type === 'response'
    && data.id !== undefined
    && data.id >= 0;
}
