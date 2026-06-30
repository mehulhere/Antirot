import fs from "node:fs";

const files = {
    web: fs.readFileSync("apps/frontend/app/page.tsx", "utf8"),
    iosModels: fs.readFileSync("apps/ios/AntirotAlarm/Sources/Models.swift", "utf8"),
    iosPlan: fs.readFileSync("apps/ios/AntirotAlarm/Sources/PlanView.swift", "utf8"),
    android: fs.readFileSync("apps/android/app/src/main/java/com/mehulhere/antirot/CoachQuickAction.java", "utf8")
};

const expectations = [
    [
        "web quick action states",
        files.web.includes(`const quickMessagesByState: Record<RuntimeStateName, string[]> = {
    onboarding: [],
    idle: ["ready-work"],
    working: ["done"],
    break: [],
    sleeping: [],
    vacation: [],
    unknown: []
};`)
    ],
    [
        "web direct action states",
        files.web.includes(`const actionsByState: Record<RuntimeStateName, string[]> = {
    onboarding: [],
    idle: ["start-work"],
    working: ["done"],
    break: [],
    sleeping: [],
    vacation: [],
    unknown: []
};`)
    ],
    [
        "ios coach quick action states",
        files.iosModels.includes(`case "onboarding":
            ids = []
        case "idle":
            ids = ["start_working"]
        case "working":
            ids = ["done"]
        case "break", "sleeping", "vacation", "unknown":
            ids = []`)
    ],
    [
        "ios plan state actions",
        files.iosPlan.includes(`switch coach.runtimeState.lowercased() {
        case "idle":`) &&
            files.iosPlan.includes(`id: "plan_start_work",
                    title: "Start Work"`) &&
            files.iosPlan.includes(`case "working":`) &&
            files.iosPlan.includes(`id: "plan_done",
                    title: "Done"`) &&
            !files.iosPlan.includes(`planButton("Break"`)
    ],
    [
        "android coach quick action states",
        files.android.includes(`case "onboarding":
                ids = new String[] {};
                break;
            case "idle":
                ids = new String[] {"start_working"};
                break;
            case "working":
                ids = new String[] {"done"};
                break;
            default:
                ids = new String[] {};`)
    ]
];

const failures = expectations.filter(([, passed]) => !passed);

if (failures.length > 0) {
    console.error("Action button map regression failed:");
    for (const [name] of failures) {
        console.error(`- ${name}`);
    }
    process.exit(1);
}

console.log("Action button maps match idle=start only and working=done only.");
