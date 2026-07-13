import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const launcher = fs.readFileSync(path.join(repoRoot, "scripts/run-frontend-lab.mjs"), "utf8");
const frontend = fs.readFileSync(path.join(repoRoot, "apps/frontend/app/page.tsx"), "utf8");
const orientation = fs.readFileSync(path.join(repoRoot, "readme_agent.md"), "utf8");

assert.doesNotMatch(launcher, /ANTIROT_ADMIN_TOKEN|ANTIROT_DEVICE_TOKEN|readVpsEnv|ssh-keyscan|backend\.env/u, "frontend launcher must never retrieve or expose a backend secret");
assert.doesNotMatch(frontend, /NEXT_PUBLIC_ANTIROT_(?:ADMIN|DEVICE)_TOKEN|\/v1\/admin|\/v1\/test/u, "browser code must use only user-scoped production endpoints");
assert.doesNotMatch(orientation, /NEXT_PUBLIC_ANTIROT_(?:ADMIN|DEVICE)_TOKEN/u, "agent guidance must never recommend browser-visible backend secrets");
assert.match(frontend, /window\.localStorage\.getItem\(DEVICE_TOKEN_STORAGE_KEY\) \|\| ""/u, "authenticated browser requests must use the user-scoped sign-in token");
assert.match(
    launcher,
    /\["next", "dev", "-H", "127\.0\.0\.1", "-p", port\]/u,
    "credential-bearing development server must bind to loopback only"
);

console.log("Frontend secret boundary checks passed.");
