// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Provider 列目录/打开失败的友好错误映射。
 *
 * 后端（rclone / 网关）给出的多是英文 stderr 原文，例如 alist 停机时的
 * `Propfind ... connectex: No connection could be made ... actively refused it`。
 * 这里只做无害的字符串分类，原文始终保留在界面上可展开，绝不吞掉细节。
 */

export type ProviderErrorKind =
  | "searchBlocked"
  | "rcloneMissing"
  | "notConfigured"
  | "connectionRefused"
  | "timeout"
  | "auth"
  | "notFound"
  | "generic";

type Rule = { kind: ProviderErrorKind; pattern: RegExp };

/** 顺序敏感：越具体的规则放越前面，generic 兜底。 */
const RULES: Rule[] = [
  { kind: "searchBlocked", pattern: /^PROVIDER_SEARCH_BLOCKED$/ },
  { kind: "rcloneMissing", pattern: /rclone not found/i },
  { kind: "notConfigured", pattern: /no rclone remotes|no remotes configured/i },
  {
    kind: "connectionRefused",
    pattern:
      /connectex|actively refused|connection refused|econnrefused|no connection could be made|connection reset|dial tcp|network is unreachable|host is unreachable/i,
  },
  { kind: "timeout", pattern: /timed?\s*out|timeout|context deadline exceeded/i },
  {
    kind: "auth",
    pattern:
      /\b401\b|\b403\b|unauthori[sz]ed|authentication failed|permission denied|invalid creden|signature does not match/i,
  },
  { kind: "notFound", pattern: /\b404\b|not found|no such file|does not exist/i },
];

/** 把原始错误文本归入一个语义类别；空串与未知文本归为 generic。 */
export function classifyProviderError(raw: string | null | undefined): ProviderErrorKind {
  const text = (raw ?? "").trim();
  if (!text) return "generic";
  for (const rule of RULES) {
    if (rule.pattern.test(text)) return rule.kind;
  }
  return "generic";
}

/** 语义类别 → i18n key。searchBlocked 复用现有文案。 */
export function providerErrorI18nKey(kind: ProviderErrorKind): string {
  if (kind === "searchBlocked") return "providers.searchBlocked";
  return `providers.error.${kind}`;
}

/** 是否展示可展开的原文（内部错误码没有展示价值）。 */
export function shouldShowProviderErrorRaw(kind: ProviderErrorKind): boolean {
  return kind !== "searchBlocked";
}
