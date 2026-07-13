import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");
const functionBody = (source, name) => {
    const start = source.indexOf(`function ${name}(`);
    assert.notEqual(start, -1, `missing ${name}`);
    const next = source.indexOf("\nfunction ", start + 1);
    return source.slice(start, next === -1 ? source.length : next);
};
const asyncFunctionBody = (source, name) => {
    const start = source.indexOf(`async fn ${name}(`);
    assert.notEqual(start, -1, `missing ${name}`);
    const next = source.indexOf("\nasync fn ", start + 1);
    return source.slice(start, next === -1 ? source.length : next);
};

const [swiftApi, swiftModels, swiftCoach, androidApi, androidMain, androidAlarm, frontend, tester, routes, backendMain, backendAuth, backendDb, backendLlm, backendMemory, backendModels, backendApns, backendReadme, envExample] = await Promise.all([
    read("apps/ios/AntirotAlarm/Sources/APIClient.swift"),
    read("apps/ios/AntirotAlarm/Sources/Models.swift"),
    read("apps/ios/AntirotAlarm/Sources/CoachViewModel.swift"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AntirotApiClient.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/MainActivity.java"),
    read("apps/android/app/src/main/java/com/mehulhere/antirot/AlarmActivity.java"),
    read("apps/frontend/app/page.tsx"),
    read("website/tester.html"),
    read("apps/backend/src/routes.rs"),
    read("apps/backend/src/main.rs"),
    read("apps/backend/src/auth.rs"),
    read("apps/backend/src/db.rs"),
    read("apps/backend/src/llm.rs"),
    read("apps/backend/src/memory.rs"),
    read("apps/backend/src/models.rs"),
    read("apps/backend/src/apns.rs"),
    read("apps/backend/README.md"),
    read("env.example.txt")
]);

for (const [name, source] of [["iOS", swiftApi], ["Android", androidApi]]) {
    const paths = [...source.matchAll(/(?:path:\s*|request\("[A-Z]+",\s*|String path =\s*)"(\/[A-Za-z][^"\\]*)/gu)].map((match) => match[1]);
    assert.ok(paths.length > 0, `${name} path audit found no endpoints`);
    assert.deepEqual(paths.filter((path) => !path.startsWith("/v1/")), [], `${name} contains unversioned production endpoints`);
    assert.doesNotMatch(source, /\/v1\/(?:test|admin)(?:\/|\b)/u, `${name} must not depend on test/admin endpoints`);
}
assert.doesNotMatch(swiftApi, /userId:\s*String\s*=\s*"admin"/u, "iOS API client must not carry an admin identity default");
assert.match(swiftModels, /struct ChatCoachRequest[\s\S]{0,160}requestId:\s*String/u, "iOS chat payload must include an idempotency key");
assert.match(swiftCoach, /QueuedChatMessage\([\s\S]{0,180}requestId:/u, "iOS must create the request ID when queueing the message");
assert.match(swiftApi, /ChatCoachRequest\(message:\s*message,\s*requestId:\s*requestId\)/u, "iOS retries must reuse the queued request ID");
assert.match(androidMain, /QueuedChatMessage\([\s\S]{0,180}UUID\.randomUUID\(\)\.toString\(\)/u, "Android must create the request ID when queueing the message");
assert.match(androidApi, /body\.put\("requestId",\s*requestId\)/u, "Android chat payload must include the queued request ID");
assert.match(frontend, /const requestId = crypto\.randomUUID\(\)/u, "frontend must create one request ID per queued message");
assert.match(frontend, /JSON\.stringify\(\{ message:\s*trimmed,\s*requestId \}\)/u, "frontend chat payload must include its request ID");
assert.match(backendModels, /pub request_id:\s*String/u, "backend must require requestId rather than silently generating one");
assert.match(backendMain, /chat_concurrency:\s*Arc<Semaphore>/u, "chat provider calls must have a global concurrency ceiling");
assert.match(routes, /chat_concurrency[\s\S]{0,240}try_acquire_owned/u, "chat must reject overload before entering provider orchestration");
const providerSend = backendLlm.indexOf(".send()", backendLlm.indexOf("while loop_count < max_loops"));
const releasedLease = backendLlm.lastIndexOf("drop(client);", providerSend);
const reacquiredLease = backendLlm.indexOf("client = pool.get().await?;", providerSend);
assert.ok(releasedLease !== -1 && releasedLease < providerSend, "chat must release its database lease before provider network waits");
assert.ok(reacquiredLease > providerSend, "chat must reacquire a database lease only after the provider response");
assert.match(backendMain, /speech_concurrency:\s*Arc<Semaphore>/u, "paid speech calls must have a global concurrency ceiling");
assert.match(routes, /reserve_daily_provider_usage/u, "paid provider calls must reserve persistent daily quota");
assert.match(routes, /"speech_stt_bytes"[\s\S]{0,240}bytes\.len\(\)/u, "STT quota must be charged by uploaded bytes");
assert.match(routes, /"speech_tts_chars"[\s\S]{0,240}req\.text\.chars\(\)\.count\(\)/u, "TTS quota must be charged by input characters");
assert.match(backendMemory, /MAX_MEMORY_DOCUMENT_CHARS/u, "canonical memory documents must have a hard size ceiling");
assert.match(backendMemory, /MAX_USER_MEMORY_CHARS/u, "aggregate user memory must have a hard size ceiling");
assert.match(backendMemory, /next_attempt_at/u, "memory jobs must use delayed retry scheduling");
assert.match(backendMemory, /THEN 'failed'/u, "poison memory jobs must dead-letter after bounded attempts");
assert.match(backendMemory, /user_allows_server_embeddings/u, "embedding providers must respect the user's processing boundary");
assert.match(routes, /validate_byok_provider/u, "subscription writes must reject unknown BYOK providers");
assert.match(backendAuth, /pub ver:\s*i64/u, "session tokens must carry a revocation version");
assert.match(backendAuth, /pub jti:\s*String/u, "session tokens must have a unique identifier");
assert.match(backendAuth, /set_issuer/u, "session validation must bind the issuer");
assert.match(backendAuth, /set_audience/u, "session validation must bind the audience");
assert.match(backendAuth, /session_version[\s\S]{0,400}device_id/u, "session validation must check current device state");
assert.match(routes, /auth_logout[\s\S]{0,800}session_version\s*=\s*session_version\s*\+\s*1/u, "logout must revoke the server-side device session");
assert.doesNotMatch(routes, /ON CONFLICT \(email\) DO UPDATE/u, "Google sign-in must not silently link a new subject by reusable email");
assert.match(routes, /email is already registered[\s\S]{0,160}explicit account-link/u, "email collisions must require an authenticated linking flow");
assert.match(routes, /MAX_ALARM_RECONCILE_ITEMS/u, "alarm reconciliation arrays must be bounded");
assert.match(routes, /LIMIT \$2/u, "alarm cancellation tombstones must be paginated or bounded");
assert.match(routes, /attempt_count\s*=\s*attempt_count\s*\+\s*1/u, "pairing sessions must record claim attempts");
assert.match(backendDb, /MakeRustlsConnect/u, "remote PostgreSQL must use verified TLS");
assert.match(backendDb, /required_schema_objects/u, "migration baselining must verify the complete required schema");
assert.match(routes, /encrypt_byok_key/u, "BYOK keys must be encrypted before database storage");
assert.match(backendLlm, /decrypt_byok_key/u, "BYOK keys must be decrypted only at provider use");
assert.match(backendApns, /Client::builder\(\)[\s\S]{0,160}timeout/u, "APNs requests must have a hard timeout");
assert.doesNotMatch(backendApns, /body = %body/u, "APNs response bodies must not be copied into logs");
for (const handler of ["create_alarm", "record_alarm_action", "restore_memory_snapshot"]) {
    const body = asyncFunctionBody(routes, handler);
    assert.doesNotMatch(body, /process_alarm_wake_outbox_for_device/u, `${handler} must not synchronously wait for APNs after commit`);
}
const iosProject = await read("apps/ios/project.yml");
const iosEntitlements = await read("apps/ios/AntirotAlarm/Resources/AntirotAlarm.entitlements");
assert.match(iosProject, /aps-environment:\s*production/u, "TestFlight project configuration must use production APNs");
assert.match(iosEntitlements, /<string>production<\/string>/u, "exported iOS entitlement source must use production APNs");
const androidManifest = await read("apps/android/app/src/main/AndroidManifest.xml");
const androidSettings = await read("apps/android/app/src/main/java/com/mehulhere/antirot/SettingsStore.java");
const androidGradle = await read("apps/android/app/build.gradle.kts");
assert.match(androidManifest, /android:allowBackup="false"/u, "Android credentials must be excluded from backup and transfer");
assert.match(androidManifest, /android:name="\.BootReceiver"/u, "Android must restore background alarm synchronization after reboot");
assert.match(androidSettings, /EncryptedSharedPreferences/u, "Android bearer tokens must use Keystore-backed encrypted storage");
assert.match(androidSettings, /SECURE_PREFS\s*=\s*"antirot_secure"/u, "encrypted preferences must not reuse the legacy plaintext file");
assert.match(androidSettings, /legacy\.edit\(\)\.clear\(\)\.commit\(\)/u, "plaintext preferences must be deleted after migration");
assert.match(androidGradle, /play-services-auth/u, "Android production sign-in must include Google authentication");
assert.match(androidGradle, /work-runtime/u, "Android must include autonomous background alarm synchronization");
const atomicBatch = asyncFunctionBody(backendLlm, "execute_tool_batch_atomically");
assert.equal((atomicBatch.match(/client\.transaction\(\)/gu) || []).length, 1, "one provider tool batch must use one database transaction");
assert.match(atomicBatch, /for \(call, decoded\)[\s\S]*transaction\.rollback\(\)[\s\S]*transaction\.commit\(\)/u, "a failed tool must roll back the whole batch before the sole commit");
assert.match(backendLlm, /execute_tool_batch_atomically\([\s\S]{0,500}calls\.into_iter\(\)\.zip\(decoded_calls\)/u, "orchestration must submit the complete provider batch atomically");

assert.match(swiftApi, /maxAudioUploadBytes\s*=\s*25 \* 1024 \* 1024/u, "iOS must bound audio before loading it into memory");
assert.match(swiftApi, /fileSize[\s\S]{0,500}maxAudioUploadBytes/u, "iOS must reject oversized audio before Data(contentsOf:)");
assert.match(androidApi, /MAX_AUDIO_UPLOAD_BYTES\s*=\s*25L \* 1024L \* 1024L/u, "Android must share the 25 MB upload boundary");
assert.match(androidApi, /file\.length\(\)[\s\S]{0,300}MAX_AUDIO_UPLOAD_BYTES/u, "Android must reject oversized audio before streaming");

assert.doesNotMatch(androidApi + androidMain + androidAlarm, /"Failed:/u, "Android fallbacks must use the repository diagnostic format");
assert.match(androidApi, /🔴 FALLBACK: Android API request failed - Reason:/u, "Android API errors must remain visible and retryable");

assert.match(tester, /Legacy tester retired/u, "legacy tester must be an explicit retirement page");
assert.doesNotMatch(tester, /test-admin-token|SIMULATOR_ADMIN_TOKEN|fetch\s*\(/u, "retired tester must ship no credential or API client");
assert.doesNotMatch(frontend, /test-admin-token|test-device-token/u, "frontend lab source must not contain fallback credentials");

assert.match(routes, /instrument_legacy_alias/u, "legacy aliases must emit compatibility telemetry");
assert.match(routes, /x-antirot-legacy-alias/u, "legacy responses must identify compatibility traffic");
assert.match(routes, /LEGACY_ALIAS_HITS/u, "legacy aliases must have a process-level hit counter");
assert.match(routes, /DefaultBodyLimit::max\(\s*MAX_AUDIO_UPLOAD_BYTES \+ MULTIPART_OVERHEAD_BYTES/u, "speech upload body limit must be explicit and route-scoped");

for (const name of ["loadSnapshot", "loadPendingAlarms"]) {
    const body = functionBody(frontend, name);
    assert.doesNotMatch(body, /catch\s*\{/u, `${name} must not silently swallow diagnostics`);
    assert.match(body, /recordFallback/u, `${name} fallback must be recorded for diagnostics`);
}
assert.match(functionBody(frontend, "loadDiagnostics"), /Admin prompt diagnostics are intentionally unavailable/u, "browser diagnostics must not require an admin token");
assert.doesNotMatch(functionBody(frontend, "todayWorkKey"), /getUTC/u, "frontend work-log tabs must follow the user's local day");
assert.match(backendMain, /spawn_alarm_wake_worker\(state\.clone\(\)\)/u, "APNs outbox must drain without a request trigger");
assert.match(backendLlm, /ORDER BY outbox\.next_attempt_at ASC, outbox\.created_at ASC/u, "retryable APNs failures must not starve later wake effects");
assert.match(backendLlm, /THEN 'failed' ELSE 'pending'/u, "repeated APNs failures must dead-letter after bounded retries");
assert.match(backendApns, /APNs is not configured/u, "missing APNs configuration must keep the wake retryable");
assert.match(backendApns, /Apple rejected APNs wake/u, "Apple non-success responses must keep the wake retryable");
assert.match(backendDb, /pg_advisory_xact_lock/u, "migration runner must serialize concurrent startup");
assert.match(backendDb, /schema_migrations/u, "migration runner must persist applied versions");
assert.doesNotMatch(backendDb, /batch_execute\(include_str!\("\.\.\/sql\/001_init\.sql"\)\)/u, "startup must not replay monolithic baseline DDL");
const hardeningMigration = await read("apps/backend/sql/002_production_hardening.sql");
assert.match(hardeningMigration, /INSERT INTO memory_index_jobs/u, "hardening migration must backfill derived index jobs");
assert.match(hardeningMigration, /ON CONFLICT \(user_id, memory_key, content_version\) DO NOTHING/u, "repeated index backfill must preserve the one existing current-version job");
assert.match(routes, /\/v1\/test\/alarm-wake\/seed/u, "database harness must seed pending and expired wake effects without request-time draining");

assert.match(backendReadme, /Legacy alias compatibility window ends 2026-10-31/u, "backend docs must define the alias removal window");
assert.match(backendReadme, /memory index worker/u, "backend docs must describe derived indexing");
assert.match(backendReadme, /autonomous alarm wake worker/u, "backend docs must describe request-independent APNs outbox processing");
assert.match(backendReadme, /schema_migrations/u, "backend docs must describe the versioned migration ledger");
assert.match(backendReadme, /advisory lock/u, "backend docs must describe concurrent-startup migration serialization");
assert.match(backendReadme, /25 MB/u, "backend docs must document upload limits");
assert.match(envExample, /Never expose ANTIROT_ADMIN_TOKEN/u, "environment example must warn against browser-visible admin credentials");
assert.doesNotMatch(envExample, /(?:AIza|sk-[A-Za-z0-9]{12,}|BEGIN PRIVATE KEY)/u, "environment example must contain no real credential material");
assert.doesNotMatch(envExample + backendReadme, /973993815360-/u, "documentation must not embed a project-specific OAuth client identifier");

console.log("Production client and compatibility contracts passed.");
