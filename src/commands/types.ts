// SPDX-License-Identifier: GPL-3.0-or-later

/** Optional group headers for the flat Command Panel list (TZ AIA-27). */
export type CommandGroup = "Navigate" | "View" | "Actions";

export type AppCommand = {
  id: string;
  title: string;
  group: CommandGroup;
  /** Extra tokens for fuzzy match (aliases, route ids). */
  keywords?: string[];
  run: () => void | Promise<void>;
};

export type RankedCommand = {
  command: AppCommand;
  score: number;
};
