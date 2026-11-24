// SPDX-License-Identifier: GPL-3.0-or-later
// SEL-010 (已拍板 2026-11-22): Esc 三层优先级单元 — rename inline 中 →
// 取消 rename；有选中 → 清空选中；两者皆无 → 不消费交给全局。
import assert from "node:assert/strict";
import { escSemantics } from "../src/state/navigatorSelection.ts";

// 第 1 层：rename inline 活跃时优先取消 rename，无论是否有选中。
assert.equal(escSemantics(true, 0), "cancel-rename");
assert.equal(escSemantics(true, 5), "cancel-rename");

// 第 2 层：无 rename、有选中 → 清空选中。
assert.equal(escSemantics(false, 1), "clear-selection");
assert.equal(escSemantics(false, 42), "clear-selection");

// 第 3 层：无 rename、无选中 → 不消费。
assert.equal(escSemantics(false, 0), "pass-through");

console.log("esc-semantics checks OK (SEL-010 rename > clear selection > pass-through).");
