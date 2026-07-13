import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const workflowsDir = path.join(repoRoot, ".github", "workflows");
const workflowNames = (await readdir(workflowsDir)).filter((name) => name.endsWith(".yml"));

for (const name of workflowNames) {
    const source = await readFile(path.join(workflowsDir, name), "utf8");
    assert.match(source, /^permissions:\n/mu, `${name} must declare workflow permissions`);
    for (const match of source.matchAll(/uses:\s*([^\s#]+)@([^\s#]+)/gu)) {
        assert.match(match[2], /^[a-f0-9]{40}$/u, `${name} action ${match[1]} must use an immutable commit SHA`);
    }
    assert.doesNotMatch(source, /gradle-version:\s*current|brew install xcodegen/u, `${name} must pin build tools`);
}

const deploy = await readFile(path.join(workflowsDir, "deploy-backend-vps.yml"), "utf8");
assert.match(deploy, /ANTIROT_VPS_HOST_KEY/u, "deploy must use a configured host key");
assert.match(deploy, /StrictHostKeyChecking=yes/u, "deploy must require the pinned host key");
assert.doesNotMatch(deploy, /ssh-keyscan|accept-new/u, "deploy must not trust first-seen host keys");

const packageJson = JSON.parse(await readFile(path.join(repoRoot, "package.json"), "utf8"));
assert.equal(
    packageJson.scripts["test:alarm-reconciliation-contracts"],
    "node scripts/test-alarm-reconciliation-contracts.mjs",
    "alarm reconciliation contracts must be runnable through npm"
);

const website = await readFile(path.join(repoRoot, "website", "index.html"), "utf8");
assert.doesNotMatch(website, /releases\/(?:antirot\.apk|Antirot-unsigned\.ipa)/u, "website must not distribute checked-in stale binaries");
assert.match(website, /releases\/latest\/download\/antirot\.apk/u, "website must use the signed release channel");

const androidBuild = await readFile(path.join(repoRoot, "apps", "android", "app", "build.gradle.kts"), "utf8");
assert.match(androidBuild, /lockAllConfigurations\(\)/u, "Android dependency versions must be locked");
await readFile(path.join(repoRoot, "apps", "android", "app", "gradle.lockfile"), "utf8");
await readFile(path.join(repoRoot, "apps", "android", "gradle", "verification-metadata.xml"), "utf8");
for (const name of ["build-android-apk.yml", "release-android.yml"]) {
    const source = await readFile(path.join(workflowsDir, name), "utf8");
    assert.match(source, /--dependency-verification=strict/u, `${name} must enforce dependency checksums`);
}

console.log("CI, deployment, and release security contracts passed.");
