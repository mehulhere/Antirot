import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

const [backendSchema, backendRoutes, backendMemory, backendLlm, backendMain, swiftApi, swiftHome, androidApi, androidMain, frontend] = await Promise.all([
    read("apps/backend/sql/001_init.sql"),
    read("apps/backend/src/routes.rs"),
    read("apps/backend/src/memory.rs"),
    read("apps/backend/src/llm.rs"),
    read("apps/backend/src/main.rs"),
    read("apps/ios/AntirotAlarm/Sources/APIClient.swift"),
    read("apps/ios/AntirotAlarm/Sources/HomeView.swift"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java"),
    read("apps/frontend/app/page.tsx")
]);

assert.match(backendSchema, /timezone\s+TEXT\s+NOT NULL\s+DEFAULT\s+'UTC'/u, "users must persist an IANA timezone");
assert.match(backendSchema, /content_version\s+TEXT/u, "canonical memories must carry content versions");
assert.match(backendSchema, /CREATE TABLE IF NOT EXISTS memory_index_states/u, "active index generations need canonical state");
assert.match(backendSchema, /CREATE TABLE IF NOT EXISTS memory_index_jobs/u, "indexing must be derived work");

assert.match(backendRoutes, /route\("\/v1\/profile\/onboarding", post\(save_onboarding_profile\)\)/u, "backend must expose the typed onboarding endpoint");
assert.match(backendRoutes, /save_memory_canonical\(&\*transaction/u, "profile persistence must use the canonical transaction");
assert.doesNotMatch(backendRoutes, /save_onboarding_profile[\s\S]{0,3000}process_memory_index_jobs/u, "profile success must not wait for embedding providers");
assert.match(backendMemory, /pub struct UserDay/u, "one helper must own user-local day selection");
assert.match(backendMemory, /memory_index_states state/u, "search must fence visibility by active generation");
assert.match(backendMain, /spawn_memory_index_worker\(state\.clone\(\)\)/u, "direct writes need a background index drain independent of chat");
assert.match(backendMemory, /process_next_memory_index_job/u, "background indexing must claim durable jobs");
assert.doesNotMatch(backendLlm, /else if file_path == "(?:personality|user_profile|durable|longterm|shortterm|behavior|tasks|routine|sleep|achievements)\.md"/u, "descriptor registry must be the only base filename mapping");

for (const [name, source] of [["iOS", swiftApi], ["Android", androidApi], ["frontend", frontend]]) {
    assert.match(source, /\/v1\/profile\/onboarding/u, `${name} must use the typed onboarding endpoint`);
    assert.match(source, /timezone/u, `${name} must send an explicit timezone field`);
}
assert.match(swiftHome, /saveOnboardingProfile\(name: name, timezone: timezone\)/u, "iOS must pass name and timezone as typed fields");
assert.match(androidMain, /saveOnboardingProfile\(name,/u, "Android must use typed profile capture instead of hidden chat prose");
assert.match(frontend, /JSON\.stringify\(\{ name, timezone \}\)/u, "frontend must send name and timezone as fields");
const onboardingRequestIndex = frontend.indexOf("/v1/profile/onboarding");
const onboardingSuccessMarkerIndex = frontend.indexOf("ONBOARDING_NAME_SENT_STORAGE_KEY, \"true\"");
assert.ok(onboardingRequestIndex >= 0 && onboardingSuccessMarkerIndex > onboardingRequestIndex, "frontend must persist onboarding success only after the typed request succeeds");
const cachedPromptSentBody = frontend.match(/function loadCachedNamePromptSent\(\) \{([\s\S]*?)\n\}/u)?.[1] ?? "";
assert.doesNotMatch(cachedPromptSentBody, /loadCachedOnboardingName|ONBOARDING_NAME_STORAGE_KEY/u, "frontend retry state must depend only on the post-success marker");

for (const source of [backendLlm, swiftHome, androidMain, frontend]) {
    assert.doesNotMatch(source, /The user just shared their name during onboarding|Silent device timezone/u, "hidden onboarding prose must not return");
}

console.log("Memory, timezone, and onboarding source contracts passed.");
