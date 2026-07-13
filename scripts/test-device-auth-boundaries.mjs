import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const routesPath = path.join(repoRoot, "apps/backend/src/routes.rs");
const source = fs.readFileSync(routesPath, "utf8");
const authSource = fs.readFileSync(path.join(repoRoot, "apps/backend/src/auth.rs"), "utf8");

function functionBody(name) {
    const start = source.indexOf(`async fn ${name}(`);
    assert.notEqual(start, -1, `missing ${name}`);

    const next = source.indexOf("\nasync fn ", start + 1);
    return source.slice(start, next === -1 ? source.length : next);
}

const registerDevice = functionBody("register_device");
assert.match(registerDevice, /is_legacy_device_bootstrap/u);
assert.match(registerDevice, /RETURNING device_id/u);
assert.match(registerDevice, /api_token_hash IS NULL/u);
assert.match(
    registerDevice,
    /require_device_auth_for\([\s\S]*?&request\.device_id,?\s*\)/u,
    "device registration must authorize the requested device ID"
);

const pendingAlarms = functionBody("pending_alarms");
assert.match(
    pendingAlarms,
    /require_device_auth_for\([\s\S]*?&query\.device_id,?\s*\)/u,
    "pending alarm reads must authorize the requested device ID"
);

const recordAlarmAction = functionBody("record_alarm_action");
assert.match(
    recordAlarmAction,
    /require_device_auth_for\([\s\S]*?&request\.device_id,?\s*\)/u,
    "alarm actions must authorize the device ID in the request"
);

const authSession = functionBody("auth_session");
assert.match(authSession, /enforce_rate_limit\([\s\S]*?"anonymous_session"/u);
assert.match(
    authSession,
    /if !state\.config\.allow_anonymous_sessions/u,
    "anonymous account creation must be disabled unless explicitly configured"
);
assert.match(
    authSession,
    /ON CONFLICT \(device_id\) DO NOTHING/u,
    "anonymous session creation must not overwrite an existing device"
);
assert.match(
    authSession,
    /RETURNING device_id/u,
    "anonymous session creation must verify that its device insert succeeded"
);

const authGoogle = functionBody("auth_google");
assert.match(authGoogle, /enforce_rate_limit\([\s\S]*?"google_auth"/u);
assert.match(
    authGoogle,
    /WHERE devices\.user_id = EXCLUDED\.user_id/u,
    "Google sign-in must only refresh a device already owned by the same user"
);
assert.match(
    authGoogle,
    /RETURNING device_id/u,
    "Google sign-in must detect a conflicting device owner"
);

const updateSubscription = functionBody("update_subscription");
assert.match(
    updateSubscription,
    /require_admin_auth\(&headers, &state\.config\)/u,
    "subscription entitlements must only be mutable with admin authorization"
);
assert.doesNotMatch(
    updateSubscription,
    /get_user_id_from_auth/u,
    "subscription mutation must not trust a user-scoped token for entitlement changes"
);
assert.match(updateSubscription, /target_user_id/u);
assert.match(updateSubscription, /RETURNING subscription_tier/u);

assert.doesNotMatch(
    authSource,
    /require_device_auth_for[\s\S]*?constant_time_eq\(token, &config\.device_token\)/u,
    "legacy bootstrap token must never bypass exact-device authorization"
);
assert.match(
    authSource,
    /config\.allow_legacy_device_bootstrap[\s\S]*?config\.device_token/u,
    "legacy device bootstrap must be explicitly enabled"
);

const claimPairing = functionBody("claim_pairing");
assert.match(claimPairing, /enforce_rate_limit\([\s\S]*?"pairing_claim"/u);

console.log("Device authorization boundary checks passed.");
