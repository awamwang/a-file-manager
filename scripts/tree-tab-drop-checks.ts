// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { createDropHoverScheduler, DROP_HOVER_MS } from "../src/state/dropHover.ts";

assert.equal(DROP_HOVER_MS, 500);

await new Promise<void>((resolve, reject) => {
  const sched = createDropHoverScheduler(20);
  let fired = 0;
  sched.schedule("a", () => {
    fired += 1;
  });
  sched.schedule("a", () => {
    fired += 10;
  });
  setTimeout(() => {
    try {
      assert.equal(fired, 1, "same-key schedule fires once");
      let second = 0;
      sched.schedule("b", () => {
        second += 1;
      });
      sched.clear();
      setTimeout(() => {
        try {
          assert.equal(second, 0, "clear cancels pending hover");
          resolve();
        } catch (err) {
          reject(err);
        }
      }, 40);
    } catch (err) {
      reject(err);
    }
  }, 40);
});

console.log("tree-tab-drop runtime checks OK.");
