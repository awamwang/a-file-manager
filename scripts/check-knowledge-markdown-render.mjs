// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 知识页 Markdown 渲染基线门禁。
 *
 * 知识页（搜索命中 → Enter 打开的单实例 Markdown 弹窗）用 marked + DOMPurify
 * 渲染笔记正文。回归信号有两类，任一出现都算行为回归：
 *   1. **换行被吃掉**：GFM 默认把段落内软换行折成空格（marked 的 breaks: false），
 *      而知识笔记多为 Logseq 风格「一行一块」（列表项 + 缩进续行），折行后整条
 *      笔记挤成一行，日期 / 正文 / 分点全部读不出来 → 必须 breaks: true。
 *   2. **任务列表勾选框消失**：DOMPurify 的 FORBID_TAGS 含 input 时，
 *      `- [ ] 待办` 渲染出的 checkbox 被整块抹掉，只剩文字 → input 只能按
 *      「disabled checkbox」放行。
 *
 * 本门禁用仓库自带的 marked 真跑一遍基础 Markdown 语法（标题 / 列表 / 引用 /
 * 代码 / 强调 / 链接 / 图片 / 表格 / 分隔线 / 转义 / 任务列表 / 软硬换行），
 * 断言渲染产物，而不是只断言源码字符串——源码断言只用来钉住那两个开关。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { marked } from "marked";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const readFrom = (rel) => readFileSync(join(root, rel), "utf8");

const dialog = readFrom("src/components/knowledge/KnowledgeDocDialog.vue");

// --- 0. 源码开关：软换行必须保留；input 不得被整体禁用 ---
const breaks = /breaks:\s*(true|false)/.exec(dialog)?.[1];
assert.equal(
  breaks,
  "true",
  "知识页 marked 必须 breaks: true（软换行保留）；breaks: false 会把「一行一块」的笔记折成一行",
);
assert.ok(
  !/breaks:\s*false/.test(dialog),
  "知识页不得再出现 breaks: false",
);

const forbidTags = /FORBID_TAGS:\s*\[([^\]]*)\]/.exec(dialog)?.[1] ?? "";
assert.ok(
  !/"input"/.test(forbidTags),
  "FORBID_TAGS 不得再含 input（GFM 任务列表的 checkbox 会被整块抹掉）",
);
assert.ok(
  /hookEvent\.tagName\s*!==\s*"input"/.test(dialog) &&
    /type\s*!==\s*"checkbox"/.test(dialog) &&
    /hasAttribute\("disabled"\)/.test(dialog),
  "知识页必须用 uponSanitizeElement 钩子把 input 收窄到 disabled checkbox",
);

/** 与知识页一致的渲染参数（breaks 由上面的源码断言钉死为 true）。 */
const render = (src) => marked.parse(src, { async: false, gfm: true, breaks: true });

// --- 1. 换行：软换行 / 硬换行 / 列表续行 ---
assert.equal(
  render("- 王蒙\n  2022年01月09日 23:19\n  三重梦境").includes(
    "<li>王蒙<br>2022年01月09日 23:19<br>三重梦境</li>",
  ),
  true,
  "列表项的缩进续行必须渲染成 <br>（一行一块）",
);
assert.equal(
  render("第一行\n第二行").includes("<p>第一行<br>第二行</p>"),
  true,
  "段落软换行必须渲染成 <br>",
);
assert.equal(
  render("第一行  \n第二行").includes("<p>第一行<br>第二行</p>"),
  true,
  "硬换行（行尾两空格）必须渲染成 <br>",
);

// --- 2. 标题 ---
assert.ok(render("# H1\n## H2\n### H3").includes("<h1>H1</h1>"), "ATX h1 标题");
assert.ok(render("#### H4\n##### H5\n###### H6").includes("<h6>H6</h6>"), "ATX h6 标题");
assert.ok(render("Title\n=====").includes("<h1>Title</h1>"), "Setext h1 标题");
assert.ok(render("Sub\n---").includes("<h2>Sub</h2>"), "Setext h2 标题");

// --- 3. 列表 ---
assert.ok(render("- 一\n- 二").includes("<ul>\n<li>一</li>\n<li>二</li>\n</ul>"), "无序列表");
assert.ok(render("1. 一\n2. 二").includes("<ol>\n<li>一</li>\n<li>二</li>\n</ol>"), "有序列表");
assert.ok(
  render("- 一\n  - 1.1\n- 二").includes("<li>一<ul>\n<li>1.1</li>\n</ul>\n</li>"),
  "嵌套列表",
);

// --- 4. 任务列表（GFM checkbox 交给 DOMPurify 钩子收窄）---
const task = render("- [ ] 未完成\n- [x] 已完成");
assert.ok(task.includes('<input disabled="" type="checkbox">'), "未完成任务 checkbox");
assert.ok(task.includes('<input checked="" disabled="" type="checkbox">'), "已完成任务 checkbox");

// --- 5. 引用 ---
assert.ok(
  render("> 引用").includes("<blockquote>\n<p>引用</p>\n</blockquote>"),
  "引用块",
);
assert.ok(render("> 外层\n>> 内层").includes("<blockquote>\n<p>内层</p>"), "嵌套引用");

// --- 6. 代码 ---
assert.ok(
  render("```js\nconst a = 1;\n```").includes('<pre><code class="language-js">'),
  "围栏代码块带语言标注",
);
assert.ok(render("    indented\n").includes("<pre><code>indented"), "缩进代码块");
assert.ok(render("用 `code`").includes("<code>code</code>"), "行内代码");

// --- 7. 强调 ---
assert.ok(render("**粗**").includes("<strong>粗</strong>"), "粗体");
assert.ok(render("*斜*").includes("<em>斜</em>"), "斜体");
assert.ok(render("~~删~~").includes("<del>删</del>"), "删除线");

// --- 8. 链接与图片 ---
assert.ok(render("[文本](https://example.com)").includes('<a href="https://example.com">文本</a>'), "行内链接");
assert.ok(
  render('[文本][ref]\n\n[ref]: https://example.com "标题"').includes(
    '<a href="https://example.com" title="标题">文本</a>',
  ),
  "引用式链接",
);
assert.ok(render("<https://example.com>").includes('<a href="https://example.com">'), "自动链接");
assert.ok(
  render("![alt](../assets/a.jpg)").includes('<img src="../assets/a.jpg" alt="alt">'),
  "图片（相对路径原样保留，资源可达性不在本门禁范围）",
);

// --- 9. 表格 ---
const table = render("| 左 | 中 |\n|:--|:-:|\n| 1 | 2 |");
assert.ok(table.includes("<table>"), "表格");
assert.ok(table.includes('<th align="left">左</th>'), "表格表头对齐");
assert.ok(table.includes('<td align="center">2</td>'), "表格单元格对齐");

// --- 10. 分隔线 / 转义 / 实体 ---
assert.ok(render("a\n\n---\n\nb").includes("<hr>"), "分隔线");
assert.ok(render("\\*不是斜体\\*").includes("*不是斜体*"), "反斜杠转义");
assert.ok(render("&amp; &#65;").includes("&amp; A"), "实体引用");

console.log(
  "knowledge-markdown-render OK (软换行保留 + 任务列表 checkbox 放行；标题/列表/引用/代码/强调/链接/图片/表格/分隔线/转义/实体 渲染正常).",
);
