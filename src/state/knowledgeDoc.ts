// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 单实例知识弹窗状态（ADR 0017）：搜索知识文件行 Enter 打开；
 * 重复打开复用同一弹窗实例并替换内容，不叠加第二个弹窗。
 * Host: `components/knowledge/KnowledgeDocDialog.vue`。
 */

import { ref } from "vue";

export const knowledgeDocOpen = ref(false);
/** 索引根相对路径（服务端 POST /doc 契约，客户端不拼绝对路径读取）。 */
export const knowledgeDocFile = ref("");

/** 打开（或切换到新文档）：单实例语义，至多一个弹窗。 */
export function openKnowledgeDoc(file: string): void {
  knowledgeDocFile.value = file;
  knowledgeDocOpen.value = true;
}

export function closeKnowledgeDoc(): void {
  knowledgeDocOpen.value = false;
  knowledgeDocFile.value = "";
}
