// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Frontend mirror of the first-run preset identity defined in
 * `src-tauri/src/examples.rs`.
 *
 * The guided tour must spotlight the *example* Work Group, not the default one:
 * the default group's title cannot be renamed, so demonstrating rename on it
 * would contradict the step. Keep this id in sync with the Rust constant —
 * `scripts/check-example-presets.mjs` asserts both sides carry the same value.
 */
export const EXAMPLE_WORK_GROUP_ID = "work-group-example-1";
