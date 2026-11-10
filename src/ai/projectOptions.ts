// SPDX-License-Identifier: GPL-3.0-or-later
/** 供 AI 面板「添加到项目」使用的 Project Workspace 选项。 */

import { invoke } from "@tauri-apps/api/core";

export type ProjectOption = {
  slug: string;
  displayName: string;
  path: string;
};

type WorkspaceListResult = {
  projectsRoot: string;
  workspaces: Array<{
    path: string;
    project: {
      slug: string;
      displayName: string;
    };
  }>;
  activeSlug: string | null;
};

export async function listProjectOptions(): Promise<{
  options: ProjectOption[];
  activeSlug: string | null;
}> {
  const list = await invoke<WorkspaceListResult>("projects_workspace_list");
  const options = list.workspaces.map((w) => ({
    slug: w.project.slug,
    displayName: w.project.displayName,
    path: w.path,
  }));
  options.sort((a, b) =>
    a.displayName.localeCompare(b.displayName, undefined, { sensitivity: "base" }),
  );
  return { options, activeSlug: list.activeSlug };
}
