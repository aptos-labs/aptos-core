import { accountQueryKeys } from './account';
import { collectiblesQueryKeys } from './collectibles';
import { networkQueryKeys } from './network';
import { transactionQueryKeys } from './transaction';
import { getAccountResourcesQueryKey } from './useAccountResources';
import { getActivityQueryKey } from './useActivity';

export const queryKeys = {
  getAccountResources: getAccountResourcesQueryKey,
  getActivity: getActivityQueryKey,
  ...collectiblesQueryKeys,
  ...accountQueryKeys,
  ...networkQueryKeys,
  ...transactionQueryKeys,
};

export default queryKeys;
