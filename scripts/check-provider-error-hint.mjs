// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Provider 错误友好化守卫：
 * - 分类模块覆盖 alist 停机 / 超时 / 认证 / 未配置等常见英文 stderr；
 * - 原文始终可展开（分类模块不吞细节，组件渲染 <details>）；
 * - ProvidersView 与 NavigatorView 两处都接入共用横幅，旧的裸 error 输出消失；
 * - 五种语言的 providers.error.* 文案齐备。
 * 注释先剥离，避免文档行满足断言。
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

function code(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/\/\/.*$/, ""))
    .join("\n");
}

const errMod = code(read("src/state/provider/errorMessage.ts"));
const banner = code(read("src/components/common/ProviderErrorBanner.vue"));
const providersView = code(read("src/views/ProvidersView.vue"));
const navigatorView = code(read("src/views/NavigatorView.vue"));

// 1. 分类模块：关键错误类别与兜底。
for (const kind of [
  "connectionRefused",
  "timeout",
  "auth",
  "notFound",
  "rcloneMissing",
  "notConfigured",
  "generic",
  "searchBlocked",
]) {
  assert.ok(errMod.includes(`"${kind}"`), `errorMessage.ts missing kind ${kind}`);
}
for (const needle of [
  "connectex",
  "actively refused",
  "timed?\\s*out",
  "authentication failed",
  "shouldShowProviderErrorRaw",
]) {
  assert.ok(errMod.includes(needle), `errorMessage.ts missing pattern ${needle}`);
}
// 原文可展开：分类只在 searchBlocked 时隐藏，其余保留。
assert.ok(
  errMod.includes('return kind !== "searchBlocked"'),
  "raw error must stay visible for all real errors",
);

// 2. 共用横幅组件：渲染 friendly 摘要 + 可展开原文。
for (const needle of [
  "classifyProviderError(props.error)",
  "providerErrorI18nKey(kind.value)",
  "<details",
  "<code",
  "fluent-banner--error",
]) {
  assert.ok(banner.includes(needle), `ProviderErrorBanner missing ${needle}`);
}

// 3. 两处 UI 接入共用组件，旧的裸 error / 本地映射消失。
assert.ok(
  providersView.includes("ProviderErrorBanner") &&
    providersView.includes('<ProviderErrorBanner v-if="error"'),
  "ProvidersView must use ProviderErrorBanner",
);
assert.ok(
  (navigatorView.match(/<ProviderErrorBanner/g) ?? []).length >= 2,
  "NavigatorView must use ProviderErrorBanner in both panes",
);
assert.ok(!navigatorView.includes("function formatListError"), "legacy formatListError lingers");
assert.ok(
  !providersView.includes("fluent-banner--error\">{{ error }}"),
  "ProvidersView still renders raw error directly",
);

// 4. i18n 文案齐备（五种语言 × 9 个键）。
const locales = ["zh-CN", "en", "de", "fr", "ja"];
const keys = [
  "connectionRefused",
  "timeout",
  "auth",
  "notFound",
  "rcloneMissing",
  "notConfigured",
  "generic",
  "rawDetails",
  "rawGeneric",
];
for (const loc of locales) {
  const catalog = JSON.parse(read(`src/i18n/locales/${loc}.json`));
  const errorTexts = catalog.providers?.error ?? {};
  for (const key of keys) {
    assert.ok(
      typeof errorTexts[key] === "string" && errorTexts[key].trim().length > 0,
      `${loc}.json missing providers.error.${key}`,
    );
  }
}

// Behavioral unit: execute the real (order-sensitive) classifier, not just grep its source.
const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/provider-error-message-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "provider-error-message behavior checks failed");

console.log("provider-error-hint checks passed");
