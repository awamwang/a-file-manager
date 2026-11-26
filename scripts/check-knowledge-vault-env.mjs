// SPDX-License-Identifier: GPL-3.0-or-later
// Managed knowledge pack must wire the vault (secret) channel end to end:
// pack.json env placeholders -> AFM settings keys -> settings defaults + migration
// -> i18n field labels in every locale. Regression of any link silently kills
// vault reveal/copy in the managed awam.knowledge extension (503 vault_not_configured).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pack = JSON.parse(
  readFileSync(join(root, "packs/knowledge/pack.json"), "utf8"),
);
const defaults = readFileSync(
  join(root, "src-tauri/src/settings/defaults.rs"),
  "utf8",
);
const migrate = readFileSync(
  join(root, "src-tauri/src/settings/migrate.rs"),
  "utf8",
);
const types = readFileSync(
  join(root, "src-tauri/src/settings/types.rs"),
  "utf8",
);

// 1. Sidecar env: vault vars resolve from AFM settings placeholders (secrets
//    never land in pack.json itself).
const sidecar = pack.contributions.sidecars.find((s) => s.id === "awam-knowledge-http");
assert.ok(sidecar, "knowledge sidecar contribution missing");
const expectedEnv = {
  KM_SERVICE_TOKEN: "${afm:settings:power.awamKnowledgeServiceToken}",
  KM_SECRET_SCRIPT: "${afm:settings:power.awamKnowledgeSecretScript}",
  KM_SECRET_AGENT_TOKEN: "${afm:settings:power.awamKnowledgeSecretAgentToken}",
  KM_SECRET_ID: "${afm:settings:power.awamKnowledgeSecretId}",
};
for (const [envName, placeholder] of Object.entries(expectedEnv)) {
  assert.equal(sidecar.env[envName], placeholder, `sidecar env ${envName}`);
}

// 2. Pack config surface: the three vault settings are user-configurable.
const configKeys = pack.contributions.config.map((c) => c.settingKey);
for (const key of Object.values(expectedEnv)
  .map((p) => p.replace("${afm:settings:", "").replace("}", ""))) {
  assert.ok(configKeys.includes(key), `pack config missing ${key}`);
}

// 3. Backend: settings defaults + schema v16 migration step exist.
for (const key of [
  "power.awamKnowledgeSecretScript",
  "power.awamKnowledgeSecretAgentToken",
  "power.awamKnowledgeSecretId",
]) {
  assert.ok(defaults.includes(`"${key}"`), `defaults.rs missing ${key}`);
  assert.ok(migrate.includes("SecretScript/SecretAgentToken/SecretId (schema v16)"));
}
// The vault keys landed in v16; later schema bumps must not break this gate.
const schemaVersion = Number(
  types.match(/SETTINGS_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/)?.[1] ?? 0,
);
assert.ok(schemaVersion >= 16, `schema version must be at least v16 (got v${schemaVersion})`);

// 4. i18n: field labels exist in every shipped locale.
const locales = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
for (const locale of locales) {
  const doc = JSON.parse(
    readFileSync(join(root, `src/i18n/locales/${locale}.json`), "utf8"),
  );
  for (const field of [
    "awamKnowledgeSecretScript",
    "awamKnowledgeSecretAgentToken",
    "awamKnowledgeSecretId",
  ]) {
    const node = doc.settings?.fields?.power?.[field];
    assert.equal(typeof node?.title, "string", `${locale} ${field}.title`);
    assert.equal(typeof node?.description, "string", `${locale} ${field}.description`);
    assert.ok(node.title.length > 0, `${locale} ${field}.title empty`);
    assert.ok(node.description.length > 0, `${locale} ${field}.description empty`);
  }
}

console.log("knowledge-vault-env OK.");
