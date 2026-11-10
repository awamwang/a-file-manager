// SPDX-License-Identifier: GPL-3.0-or-later
/** Visible Work Group name. The default group keeps a fixed label. */

export function workGroupDisplayName(
  group: { name: string; isDefault: boolean },
  defaultName: string,
): string {
  return group.isDefault ? defaultName : group.name;
}
