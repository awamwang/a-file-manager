// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure read of `core.showHidden`. Default is off. */

export function readShowHidden(values: Record<string, unknown> | null | undefined): boolean {
  return values?.["core.showHidden"] === true;
}
