// SPDX-License-Identifier: GPL-3.0-or-later
/** 搜索源注册表：命令 / 文件 / 项目 / 工作组 / 工具 / 扩展（Pack 静态条目）。 */

import type { Router } from "vue-router";
import type { AppCommand } from "../commands/types.ts";
import { createCommandsSource } from "./sources/commands.ts";
import { createExtensionsSource } from "./sources/extensions.ts";
import { createFilesSource } from "./sources/files.ts";
import { createProjectsSource } from "./sources/projects.ts";
import { createToolsSource } from "./sources/tools.ts";
import { createWorkgroupsSource } from "./sources/workgroups.ts";
import type { SearchSource } from "./types.ts";

export type SearchPaletteDeps = {
  router: Router;
  /** 当前命令列表（locale / Pack 变化时由壳层重算后经 getter 取新值）。 */
  commands: () => readonly AppCommand[];
  /** labelKey 翻译（扩展源标题）。 */
  t: (key: string) => string;
};

export function buildSearchSources(deps: SearchPaletteDeps): SearchSource[] {
  return [
    createCommandsSource(deps.commands),
    createFilesSource(deps.router),
    createProjectsSource(deps.router),
    createWorkgroupsSource(),
    createToolsSource(deps.router),
    createExtensionsSource({ router: deps.router, t: deps.t }),
  ];
}
