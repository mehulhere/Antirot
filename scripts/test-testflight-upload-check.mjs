#!/usr/bin/env node

import assert from "node:assert/strict";

import {
    evaluateTestFlightUpload,
    excerptErrorLog,
    parseArgs
} from "./check-testflight-upload.mjs";

const successfulRun = {
    databaseId: 123,
    displayTitle: "Deploy iOS TestFlight",
    headBranch: "main",
    status: "completed",
    conclusion: "success",
    url: "https://github.com/mehulhere/Antirot/actions/runs/123",
    jobs: [
        {
            name: "Deploy to TestFlight",
            conclusion: "success",
            steps: [
                { name: "Checkout Repository", conclusion: "success" },
                { name: "Upload to TestFlight", conclusion: "success" },
                { name: "Clean Up Credentials", conclusion: "success" }
            ]
        }
    ]
};

const failedUploadRun = {
    ...successfulRun,
    databaseId: 124,
    conclusion: "failure",
    jobs: [
        {
            name: "Deploy to TestFlight",
            conclusion: "failure",
            steps: [
                { name: "Checkout Repository", conclusion: "success" },
                { name: "Upload to TestFlight", conclusion: "failure" }
            ]
        }
    ]
};

const missingUploadStepRun = {
    ...successfulRun,
    databaseId: 125,
    jobs: [
        {
            name: "Deploy to TestFlight",
            conclusion: "success",
            steps: [
                { name: "Checkout Repository", conclusion: "success" }
            ]
        }
    ]
};

const skippedUploadRun = {
    ...successfulRun,
    databaseId: 126,
    conclusion: "failure",
    jobs: [
        {
            name: "Deploy to TestFlight",
            conclusion: "failure",
            steps: [
                { name: "Compile app without signing", conclusion: "failure" },
                { name: "Upload to TestFlight", conclusion: "skipped" }
            ]
        }
    ]
};

const runningRun = {
    ...successfulRun,
    status: "in_progress",
    conclusion: null
};

const longErrorLog = `${"a".repeat(140)}first ^ marker${"b".repeat(260)}last ^ marker${"c".repeat(140)}`;
const xcodeAssetFailureLog = [
    "CompileSwift normal arm64 (in target 'Antirot' from project 'Antirot')",
    `${"caret context ".repeat(20)}^ unrelated compiler caret`,
    "Deploy to TestFlight\tCompile app without signing\t2026-07-06T17:22:19.4040710Z SwiftDriver Antirot normal arm64 com.apple.xcode.tools.swift.compiler -serialize-diagnostics -no-color-diagnostics /DerivedData/Build/Intermediates.noindex/ExplicitPrecompiledModules/Symbols.scan",
    "quiet build output",
    "CompileAssetCatalogVariant thinned Antirot.app Assets.xcassets",
    "/* com.apple.actool.errors */",
    "Deploy to TestFlight\tCompile app without signing\t2026-07-06T17:22:20.4040710Z AntirotAlarm/Resources/Assets.xcassets: error: Failed to find a suitable device for the type IBSimDeviceTypeiPad3x with runtime iOS 26.5",
    "Failure Reason: Failed to create new simulator device in set SimDeviceSet.",
    "AntirotAlarm/Resources/Assets.xcassets: error: Failed to write info plist component.",
    "error: The file “Antirot-dependencies-1.json” couldn’t be saved in the folder “arm64”.",
    "** BUILD FAILED **",
    "The following build commands failed:",
    "CompileAssetCatalogVariant thinned Antirot.app Assets.xcassets",
    "SwiftDriver Antirot normal arm64 com.apple.xcode.tools.swift.compiler",
    "##[error]Process completed with exit code 65."
].join("\n");

assert.deepEqual(evaluateTestFlightUpload(successfulRun), {
    ok: true,
    runId: 123,
    runUrl: "https://github.com/mehulhere/Antirot/actions/runs/123",
    status: "succeeded",
    message: "Upload to TestFlight succeeded."
});

assert.deepEqual(evaluateTestFlightUpload(runningRun), {
    ok: false,
    runId: 123,
    runUrl: "https://github.com/mehulhere/Antirot/actions/runs/123",
    status: "running",
    message: "running"
});

const failedUploadResult = evaluateTestFlightUpload(failedUploadRun, longErrorLog);
assert.equal(failedUploadResult.ok, false);
assert.equal(failedUploadResult.status, "error");
assert.match(failedUploadResult.message, /Upload to TestFlight step concluded with failure/);
assert.equal(failedUploadResult.error, excerptErrorLog(longErrorLog));
assert.equal(failedUploadResult.error.startsWith("a"), true);
assert.equal(failedUploadResult.error.endsWith("c"), true);
assert.equal(failedUploadResult.error.length <= 420, true);
assert.match(failedUploadResult.error, /first \^ marker/);
assert.match(failedUploadResult.error, /last \^ marker/);
assert.match(failedUploadResult.error, /\n\.\.\.\n/);

const xcodeAssetExcerpt = excerptErrorLog(xcodeAssetFailureLog);
assert.match(xcodeAssetExcerpt, /Assets\.xcassets: error: Failed to find a suitable device/);
assert.match(xcodeAssetExcerpt, /Antirot-dependencies-1\.json/);
assert.match(xcodeAssetExcerpt, /BUILD FAILED/);
assert.doesNotMatch(xcodeAssetExcerpt, /unrelated compiler caret/);
assert.doesNotMatch(xcodeAssetExcerpt, /serialize-diagnostics/);
assert.doesNotMatch(xcodeAssetExcerpt, /Deploy to TestFlight\tCompile app without signing/);

const skippedUploadResult = evaluateTestFlightUpload(skippedUploadRun, xcodeAssetFailureLog);
assert.equal(skippedUploadResult.ok, false);
assert.match(skippedUploadResult.message, /Build failed before Upload to TestFlight/);
assert.match(skippedUploadResult.error, /Assets\.xcassets: error/);

assert.equal(evaluateTestFlightUpload(missingUploadStepRun).ok, false);
assert.equal(evaluateTestFlightUpload(missingUploadStepRun).status, "error");
assert.match(
    evaluateTestFlightUpload(missingUploadStepRun).message,
    /Upload to TestFlight step was not found/
);

assert.equal(excerptErrorLog("short error without caret"), "short error without caret");

assert.deepEqual(parseArgs(["--run-id", "456", "--workflow", "custom.yml", "--branch", "release"]), {
    branch: "release",
    json: false,
    pollIntervalSeconds: 30,
    repo: null,
    runId: "456",
    timeoutSeconds: 1800,
    wait: true,
    workflow: "custom.yml"
});

assert.deepEqual(parseArgs(["--no-wait", "--poll-interval", "5", "--timeout", "60"]), {
    branch: "main",
    json: false,
    pollIntervalSeconds: 5,
    repo: null,
    runId: null,
    timeoutSeconds: 60,
    wait: false,
    workflow: "deploy-ios-testflight.yml"
});

console.log("TestFlight upload checker tests passed.");
