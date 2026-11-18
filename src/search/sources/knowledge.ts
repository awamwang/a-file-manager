// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 知识内容源（ADR 0016 / ADR 0017 / glossary「知识内容源」）：
 * - 经 awam-knowledge HTTP 服务（Rust `knowledge_service_search`）做 BM25 内容检索；
 * - 服务端 dedup=file：按文件去重出单行（每文件取最高分 chunk，再 topk）；
 * - 文件行 Enter → 打开单实例知识弹窗（openKnowledgeDoc，markdown 渲染 + 掩码 chip 交互）；
 *   Ctrl+Enter → 资源管理器定位（响应 root 拼绝对路径，ADR 0017）；
 * - 密钥取用不在搜索面板进行（ADR 0017：密钥行已移除），改由知识弹窗 chip 承载；
 * - 服务离线 / 鉴权失败出状态行，不阻塞其它源。
 */

import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { openKnowledgeDoc } from "../../state/knowledgeDoc.ts";
import { requestShellIcons } from "../../state/shellIcons.ts";
import type {
  SearchActivateOutcome,
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";

/** 带内类型：与服务契约（camelCase）一致。 */
type KnowledgeHit = {
  path: string;
  title: string;
  line: number;
  lines: string;
  score: number;
  snippet: string;
};

type KnowledgeSearchResponse = {
  hits: KnowledgeHit[];
  total: number;
  /** 索引根绝对路径（dedup=file 响应附带，供拼绝对路径）。 */
  root?: string | null;
  index?: {
    root?: string | null;
    docs?: number | null;
    chunks?: number | null;
  } | null;
};

/** 输入防抖：HTTP 检索每键只发一次。 */
const DEBOUNCE_MS = 180;
/** 文件行上限（dedup=file 后每行即一个文件）。 */
const HIT_LIMIT = 8;

let epoch = 0;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function normalizeHitPath(path: string): string {
  return path.replace(/\//g, "\\");
}

/** 索引根：响应 root 优先，回退 index.root。 */
function responseRoot(result: KnowledgeSearchResponse): string | null {
  return result.root ?? result.index?.root ?? null;
}

export function createKnowledgeSource(): SearchSource {
  return {
    id: "knowledge",
    labelKey: "searchPalette.scopes.knowledge",
    enabled: () => true,
    unavailableKey: "searchPalette.status.knowledgeUnavailable",
    async search(query: string): Promise<SearchSourceResult> {
      const seq = ++epoch;
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      await sleep(DEBOUNCE_MS);
      if (seq !== epoch) return { status: "ready", items: [] };
      let result: KnowledgeSearchResponse;
      try {
        result = await invoke<KnowledgeSearchResponse>(
          "knowledge_service_search",
          { query: q, topk: HIT_LIMIT, dedup: "file" },
        );
      } catch {
        // 服务离线 / 鉴权失败 / 超时：出状态行，不阻塞其它源。
        return { status: "unavailable", items: [] };
      }
      if (seq !== epoch) return { status: "ready", items: [] };
      const root = responseRoot(result);
      const hits = result.hits.slice(0, HIT_LIMIT);
      const absPaths = root
        ? hits.map((hit) => `${root}\\${normalizeHitPath(hit.path)}`)
        : [];
      if (absPaths.length) requestShellIcons(absPaths);
      const items: SearchResultItem[] = hits.map((hit) => {
        const relPath = normalizeHitPath(hit.path);
        const absPath = root ? `${root}\\${relPath}` : relPath;
        return {
          key: `knowledge:note:${hit.path}`,
          sourceId: "knowledge",
          type: "file",
          title: hit.title,
          subtitle: hit.snippet,
          iconPath: absPath,
          activate: async ({ ctrl }): Promise<SearchActivateOutcome> => {
            if (ctrl) {
              void revealItemInDir(absPath);
              return;
            }
            openKnowledgeDoc(hit.path);
          },
        };
      });
      return { status: "ready", items };
    },
  };
}
