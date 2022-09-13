import { DappInfo } from './dapp-info';
import { Permission } from './permission';

export interface PermissionRequest {
  dappInfo: DappInfo;
  id: number;
  permission: Permission
}

export function isPermissionRequest(request?: PermissionRequest): request is PermissionRequest {
  return request !== undefined
    && request.dappInfo !== undefined
    && request.id !== undefined
    && request.permission !== undefined;
}
