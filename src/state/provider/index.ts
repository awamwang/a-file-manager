// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 路径浏览：虚拟 URI / 列表映射 / IPC 桥。 */

export {
  PROVIDER_ROOT,
  type ProviderQueryParam,
  getProviderQuery,
  isBlockedSearchProvider,
  isProviderPath,
  normalizeProviderKey,
  providerBackendId,
  toProviderUri,
  providerParentPath,
  providerLeafName,
  withProviderQuery,
} from "./path";
export {
  isSoftProviderQueryAwaiting,
  providerListingFromDir,
  providerRootEntriesFromBackends,
  shouldThrowProviderListError,
  type ProviderBackendInfo,
  type ProviderDirListingWire,
  type ProviderEntryWire,
  type ProviderNavigatorEntry,
  type ProviderNavigatorListing,
} from "./listing";
export {
  listProviderBackends,
  loadProviderListing,
  openProviderEntry,
  type ProviderResolveResult,
} from "./browse";
