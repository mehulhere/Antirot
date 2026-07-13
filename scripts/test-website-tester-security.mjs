import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const tester = fs.readFileSync(path.join(repoRoot, "website/tester.html"), "utf8");

assert.match(tester, /Legacy tester retired/u, "tester must remain an explicit retirement page");
assert.doesNotMatch(tester, /<script|fetch\s*\(|XMLHttpRequest|WebSocket/u, "retired tester must execute no client networking code");
assert.doesNotMatch(tester, /test-admin-token|Authorization|Bearer|apiToken|adminToken/u, "retired tester must contain no credential material");
assert.doesNotMatch(tester, /<form|<input|<button/u, "retired tester must expose no stale interactive controls");

console.log("Website tester retirement security checks passed.");
