// Prune stale Cargo 1.100 target artifacts without invalidating live fingerprints.
// Dry-run by default; --apply moves candidates to ~/.Trash (recoverable).
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  statSync,
} from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const debugDir = join(repoRoot, "src-tauri", "target", "debug");

const args = process.argv.slice(2);
const apply = args.includes("--apply");
const keepIncArg = args.find((a) => a.startsWith("--keep-incremental="));
const keepIncremental = keepIncArg ? Number(keepIncArg.split("=")[1]) : 3;
if (!Number.isInteger(keepIncremental) || keepIncremental < 1) {
  console.error("--keep-incremental must be a positive integer");
  process.exit(2);
}

function walkFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walkFiles(p));
    else if (entry.isFile()) out.push(p);
  }
  return out;
}

function stable(value) {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).sort()) out[key] = stable(value[key]);
    return out;
  }
  return value;
}

function newestMtime(files) {
  let max = 0;
  for (const f of files) max = Math.max(max, statSync(f).mtimeMs);
  return max;
}

function duKb(paths) {
  let total = 0;
  const chunk = 200;
  for (let i = 0; i < paths.length; i += chunk) {
    const batch = paths.slice(i, i + chunk);
    if (!batch.length) continue;
    const out = execFileSync("du", ["-sk", ...batch], {
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
    });
    for (const line of out.split("\n")) {
      if (!line.trim()) continue;
      total += Number(line.split(/\s+/, 1)[0]);
    }
  }
  return total;
}

if (!existsSync(debugDir)) {
  console.error(`Missing ${debugDir}; run cargo build first.`);
  process.exit(2);
}

// 1. build/<crate>/<hash>: keep the newest dir per exact fingerprint signature.
const buildRoot = join(debugDir, "build");
const allBuild = [];
const groups = new Map();
if (existsSync(buildRoot)) {
  for (const crate of readdirSync(buildRoot)) {
    const crateDir = join(buildRoot, crate);
    if (!statSync(crateDir).isDirectory()) continue;
    for (const hash of readdirSync(crateDir)) {
      const dir = join(crateDir, hash);
      if (!statSync(dir).isDirectory()) continue;
      allBuild.push(dir);

      const fpDir = join(dir, "fingerprint");
      let fps = [];
      if (existsSync(fpDir)) {
        fps = readdirSync(fpDir).filter((f) => f.endsWith(".json")).map((f) => join(fpDir, f));
      }
      if (!fps.length) continue; // no fingerprint: always stale

      const mtime = newestMtime(fps);
      for (const fp of fps) {
        let doc;
        try {
          doc = JSON.parse(readFileSync(fp, "utf8"));
        } catch {
          continue;
        }
        const key = `${crate}|${basename(fp)}|${JSON.stringify(stable(doc))}`;
        const entries = groups.get(key) ?? [];
        entries.push({ mtime, dir });
        groups.set(key, entries);
      }
    }
  }
}
const keep = new Set();
for (const entries of groups.values()) {
  entries.sort((a, b) => b.mtime - a.mtime);
  keep.add(entries[0].dir);
}
const staleBuild = allBuild.filter((d) => !keep.has(d));

// 2. incremental/<crate>-<hash>: keep the newest N dirs per crate.
const incRoot = join(debugDir, "incremental");
const allInc = [];
const incGroups = new Map();
if (existsSync(incRoot)) {
  for (const name of readdirSync(incRoot)) {
    const dir = join(incRoot, name);
    if (!statSync(dir).isDirectory()) continue;
    allInc.push(dir);
    const crate = name.replace(/-[^-]+$/, "");
    const entries = incGroups.get(crate) ?? [];
    entries.push({ mtime: newestMtime(walkFiles(dir)), dir });
    incGroups.set(crate, entries);
  }
}
const keepInc = new Set();
for (const entries of incGroups.values()) {
  entries.sort((a, b) => b.mtime - a.mtime);
  for (const e of entries.slice(0, keepIncremental)) keepInc.add(e.dir);
}
const staleInc = allInc.filter((d) => !keepInc.has(d));

const stale = [...staleBuild, ...staleInc];
if (!stale.length) {
  console.log("Nothing to prune.");
  process.exit(0);
}

const kb = duKb(stale);
console.log(`Candidates: ${stale.length} dirs (${(kb / 1024 / 1024).toFixed(2)} GiB)`);
console.log(`  build: ${staleBuild.length}; incremental: ${staleInc.length}`);

if (!apply) {
  console.log("Dry run. Re-run with --apply to move them to ~/.Trash.");
  process.exit(0);
}

const stamp = new Date().toISOString().replace(/[-:T]/g, "").slice(0, 14);
const trash = join(homedir(), ".Trash", `a-file-manager-target-prune-${stamp}`);
mkdirSync(trash, { recursive: true });
let moved = 0;
let skipped = 0;
for (const src of stale) {
  const dst = join(trash, relative(debugDir, src));
  if (existsSync(dst)) {
    skipped += 1;
    continue;
  }
  mkdirSync(dirname(dst), { recursive: true });
  renameSync(src, dst);
  moved += 1;
}
console.log(`Moved ${moved} dirs to ${trash} (skipped ${skipped}).`);
console.log("Empty the Trash to reclaim disk space.");
