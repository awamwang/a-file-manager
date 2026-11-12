// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  getProviderQuery,
  isBlockedSearchProvider,
  isProviderPath,
  PROVIDER_ROOT,
  providerBackendId,
  providerLeafName,
  providerParentPath,
  toProviderUri,
  withProviderQuery,
} from "../src/state/provider/path.ts";
import {
  providerListingFromDir,
  providerRootEntriesFromBackends,
} from "../src/state/provider/listing.ts";
import {
  buildNavigatorBreadcrumbs,
  buildProviderBreadcrumbs,
  navigableParentPath,
} from "../src/state/navHistory.ts";
import { GUIDE_PATH } from "../src/state/guide/path.ts";
import { createDirectoryTab, displayNameFromPath } from "../src/state/openState/model.ts";

assert.equal(PROVIDER_ROOT, "awam://root");
assert.equal(isProviderPath("awam://rclone"), true);
assert.equal(isProviderPath("awam://rclone/mysftp"), true);
assert.equal(isProviderPath("afm://guide"), false);
assert.equal(isProviderPath("C:\\Temp"), false);

assert.equal(toProviderUri("awam://rclone/"), "awam://rclone");
assert.equal(toProviderUri("awam://"), PROVIDER_ROOT);
// browser Profile 段大小写与 %20 必须保留，否则 Unknown profile。
assert.equal(
  toProviderUri("awam://browser/edge/Profile%201"),
  "awam://browser/edge/Profile%201",
);
assert.equal(
  toProviderUri("awam://Browser/edge/Profile%201"),
  "awam://browser/edge/Profile%201",
);
assert.notEqual(
  toProviderUri("awam://browser/edge/Profile%201"),
  "awam://browser/edge/profile%201",
);
assert.equal(
  toProviderUri("awam://prowlarr/search?q=ubuntu"),
  "awam://prowlarr/search?q=ubuntu",
);
assert.equal(providerBackendId("awam://Search/everything?q=a"), "search");
assert.equal(isBlockedSearchProvider("awam://search/everything?q=a"), true);
assert.equal(isBlockedSearchProvider("awam://prowlarr/search?q=a"), false);
assert.deepEqual(getProviderQuery("awam://prowlarr?q=ubuntu"), { q: "ubuntu" });
assert.equal(providerParentPath("awam://hf/remote?q=bert"), "awam://hf");
assert.equal(
  withProviderQuery("awam://hf/remote", { q: "bert" }),
  "awam://hf/remote?q=bert",
);
assert.equal(
  withProviderQuery("awam://hf/remote?q=bert", { q: null }),
  "awam://hf/remote",
);
assert.equal(providerParentPath(PROVIDER_ROOT), null);
assert.equal(providerParentPath("awam://rclone"), PROVIDER_ROOT);
assert.equal(providerParentPath("awam://rclone/a/b"), "awam://rclone/a");
assert.equal(providerLeafName("awam://rclone/mysftp"), "mysftp");
assert.equal(providerLeafName(PROVIDER_ROOT), "root");

const listing = providerListingFromDir({
  uri: "awam://rclone",
  backend: "rclone",
  entries: [
    {
      name: "mysftp",
      uri: "awam://rclone/mysftp",
      isDir: true,
      isFile: false,
      size: 0,
    },
    {
      name: "readme.txt",
      uri: "awam://rclone/readme.txt",
      isDir: false,
      isFile: true,
      size: 12,
      modifiedTime: 1_700_000_000_000,
    },
  ],
});
assert.equal(listing.path, "awam://rclone");
assert.equal(listing.dirCount, 1);
assert.equal(listing.fileCount, 1);
assert.equal(listing.entries[0]?.path, "awam://rclone/mysftp");
assert.equal(listing.entries[1]?.kind, "file");

const roots = providerRootEntriesFromBackends([
  {
    id: "knowledge",
    name: "Knowledge",
    description: "notes",
    healthy: true,
    capabilities: ["list"],
  },
]);
assert.equal(roots[0]?.path, "awam://knowledge");

const crumbs = buildProviderBreadcrumbs("awam://rclone/a/b");
assert.deepEqual(
  crumbs.map((c) => c.path),
  [PROVIDER_ROOT, "awam://rclone", "awam://rclone/a", "awam://rclone/a/b"],
);
assert.equal(navigableParentPath("awam://rclone/a/b"), "awam://rclone/a");
assert.equal(navigableParentPath("awam://rclone"), PROVIDER_ROOT);
assert.equal(navigableParentPath(PROVIDER_ROOT), GUIDE_PATH);

const withRoot = buildNavigatorBreadcrumbs("awam://rclone", "This PC");
assert.equal(withRoot[0]?.path, GUIDE_PATH);
assert.equal(withRoot[1]?.path, PROVIDER_ROOT);
assert.equal(withRoot[2]?.path, "awam://rclone");

assert.equal(displayNameFromPath(PROVIDER_ROOT), "Providers");
assert.equal(displayNameFromPath("awam://rclone/mysftp"), "mysftp");
const tab = createDirectoryTab("awam://knowledge");
assert.equal(tab.type, "provider");
assert.equal(tab.name, "knowledge");

console.log("provider-path checks OK");
