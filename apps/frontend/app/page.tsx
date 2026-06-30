"use client";

import {
    Activity,
    AlarmClock,
    Brain,
    Check,
    ClipboardList,
    Coffee,
    Gauge,
    HeartPulse,
    Loader2,
    Mic,
    Moon,
    Play,
    RefreshCw,
    Send,
    Square,
    Trash2,
    Volume2
} from "lucide-react";
import type { FormEvent } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { MicVAD } from "@ricky0123/vad-web";

const BACKEND_URL = process.env.NEXT_PUBLIC_ANTIROT_BACKEND_URL || "https://api.antirot.org";
const ADMIN_TOKEN = process.env.NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN || "test-admin-token";
const DEVICE_TOKEN = process.env.NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN || "test-device-token";
const GOOGLE_WEB_CLIENT_ID = process.env.NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID || "";
const USER_ID = "admin";
const DEVICE_ID = "frontend-lab-device";
const VAD_SAMPLE_RATE = 16000;
const VAD_MIN_UPLOAD_SECONDS = 10;
const VAD_PREFERRED_UPLOAD_SECONDS = 30;
const VAD_HARD_UPLOAD_SECONDS = 60;
const VAD_SETTLED_SILENCE_MS = 1500;
const REPORT_WINDOW_MS = 30 * 60 * 1000;
const ONBOARDING_NAME_STORAGE_KEY = "antirot:onboardingName";
const ONBOARDING_NAME_SENT_STORAGE_KEY = "antirot:onboardingNameSent";

type Role = "user" | "coach" | "system";
type Status = "idle" | "loading" | "ok" | "fail";
type RuntimeStateName = "onboarding" | "idle" | "working" | "break" | "sleeping" | "vacation" | "unknown";

type ChatMessage = {
    id: string;
    role: Role;
    text: string;
    at: string;
    audioUrl?: string;
    audioSeconds?: number;
};

const initialMessages: ChatMessage[] = [
    {
        id: "welcome",
        role: "system",
        text: "Antirot Lab is ready. Start the backend, then use voice, chat, or direct state actions.",
        at: "ready"
    }
];

type RuntimeState = {
    state: string;
    sourceTool: string;
    metadata: string;
};

type AlarmCount = {
    kind: string;
    severity: string;
    count: number;
};

type Snapshot = {
    ok: boolean;
    userId: string;
    deviceId: string;
    runtimeState: RuntimeState | null;
    alarmCounts: AlarmCount[];
};

type PendingAlarm = {
    id: string;
    kind: string;
    severity: string;
    title?: string;
    message?: string;
    fire_at?: string;
    fireAt?: string;
};

type MemoryResponse = {
    content: string;
    updatedAt?: string;
};

type ContextReport = {
    ok: boolean;
    userId: string;
    report: {
        provider: string;
        model: string;
        systemPromptChars: number;
        toolCount: number;
        memory: {
            totalRawChars: number;
            totalInjectedChars: number;
            totalMemoryBudgetChars: number;
            truncatedSections: string[];
        };
    };
    runtimeState?: RuntimeState | null;
    sleepMetrics?: {
        sleepSampleCount: number;
        averageSleepHours?: number;
        usualSleepHourLocal?: number;
        usualWakeHourLocal?: number;
    };
};

type ApiError = Error & { status?: number };

type GoogleCredentialResponse = {
    credential?: string;
    select_by?: string;
};

type GoogleIdConfiguration = {
    client_id: string;
    callback: (response: GoogleCredentialResponse) => void;
    ux_mode?: "popup" | "redirect";
};

type GoogleButtonOptions = {
    theme?: "outline" | "filled_blue" | "filled_black";
    size?: "large" | "medium" | "small";
    type?: "standard" | "icon";
    shape?: "rectangular" | "pill" | "circle" | "square";
    text?: "signin_with" | "signup_with" | "continue_with" | "signin";
    width?: number;
};

declare global {
    interface Window {
        google?: {
            accounts: {
                id: {
                    initialize: (config: GoogleIdConfiguration) => void;
                    renderButton: (parent: HTMLElement, options: GoogleButtonOptions) => void;
                };
            };
        };
    }
}

type LabAction = {
    id: string;
    label: string;
    icon: React.ReactNode;
    tool: string;
    args: Record<string, string | number | boolean | undefined>;
};

type QuickMessage = {
    id: string;
    label: string;
    text: string;
};

type SpeechChunkItem = {
    blob: Blob;
    reason: string;
    seconds: number;
    sequence: number;
};

type SpeechTranscriptResult = {
    text: string;
    seconds: number;
    reason: string;
    error?: string;
};

type ReportEvent = {
    id: string;
    at: string;
    kind: string;
    summary: string;
    detail?: string;
};

type CreateReportResponse = {
    ok: boolean;
    reportId: string;
    savedAt: string;
};

type ReportMemorySnapshot = {
    key: string;
    content: string;
    previous?: string;
    summary: string;
};

const memoryTabs = [
    { key: "tasks", label: "Tasks" },
    { key: "routine", label: "Routine" },
    { key: "sleep", label: "Sleep" },
    { key: "behavior", label: "Behavior" },
    { key: "durable", label: "Durable" },
    { key: "longterm", label: "Goals" },
    { key: "work", label: "Work Log" }
];

const quickMessages: QuickMessage[] = [
    {
        id: "ready-work",
        label: "I am ready to work",
        text: "I am ready to work. Start the task we just picked."
    },
    {
        id: "done",
        label: "Done",
        text: "Done. I finished the current task. Ask me how much of it was actually productive before closing it."
    },
    {
        id: "real-break",
        label: "I need a real break",
        text: "I need a real break. Help me choose the minimum honest break."
    },
    {
        id: "awake",
        label: "I am awake",
        text: "I am awake. Log it and tell me the first specific move."
    },
    {
        id: "movie-break",
        label: "Movie break check",
        text: "I want a 2 hour movie break because I deserve it. Please please."
    }
];

const quickMessagesByState: Record<RuntimeStateName, string[]> = {
    onboarding: ["ready-work"],
    idle: ["ready-work", "real-break", "movie-break"],
    working: ["done", "real-break"],
    break: ["ready-work"],
    sleeping: ["awake"],
    vacation: [],
    unknown: []
};

const actionsByState: Record<RuntimeStateName, string[]> = {
    onboarding: ["start-work"],
    idle: ["start-work", "break"],
    working: ["extend-work", "break"],
    break: ["start-work"],
    sleeping: ["wake"],
    vacation: [],
    unknown: []
};

function nowLabel() {
    return new Date().toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit"
    });
}

function todayWorkKey() {
    const today = new Date();
    const year = today.getUTCFullYear();
    const month = String(today.getUTCMonth() + 1).padStart(2, "0");
    const date = String(today.getUTCDate()).padStart(2, "0");
    return `work_log_${year}_${month}_${date}`;
}

function runtimeStateName(state?: string): RuntimeStateName {
    if (
        state === "onboarding" ||
        state === "idle" ||
        state === "working" ||
        state === "break" ||
        state === "sleeping" ||
        state === "vacation"
    ) {
        return state;
    }
    return "unknown";
}

async function backendJson<T>(path: string, init: RequestInit = {}, authToken = ADMIN_TOKEN): Promise<T> {
    const headers = new Headers(init.headers);
    if (authToken) {
        headers.set("Authorization", `Bearer ${authToken}`);
    }
    if (init.body && !(init.body instanceof FormData)) {
        headers.set("Content-Type", "application/json");
    }

    const response = await fetch(`${BACKEND_URL}${path}`, {
        ...init,
        headers
    });
    const text = await response.text();
    let body: unknown = {};
    try {
        body = text ? JSON.parse(text) : {};
    } catch {
        body = { raw: text };
    }
    if (!response.ok) {
        const error = new Error(`${response.status}: ${JSON.stringify(body).slice(0, 500)}`) as ApiError;
        error.status = response.status;
        throw error;
    }
    return body as T;
}

function concatFloat32(chunks: Float32Array[]) {
    const length = chunks.reduce((total, chunk) => total + chunk.length, 0);
    const combined = new Float32Array(length);
    let offset = 0;
    for (const chunk of chunks) {
        combined.set(chunk, offset);
        offset += chunk.length;
    }
    return combined;
}

function onboardingMessage(name: string) {
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "unknown";
    return [
        "The user just shared their name during onboarding. Return the deterministic Antirot first onboarding message exactly.",
        "Silent client context is available below for scheduling only.",
        "Do not mention timezone, profile setup, profile updates, saved fields, or that anything was saved unless the user explicitly asks.",
        "First onboarding message: I’m Antirot. I’ve coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\nSo let’s see what you’ve got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\nTell me what your day looks like and what you’re planning to get done today.",
        `Name: ${name || "not provided"}`,
        `Silent device timezone: ${timezone}`
    ].join("\n");
}

function loadCachedOnboardingName() {
    if (typeof window === "undefined") {
        return "";
    }
    return window.localStorage.getItem(ONBOARDING_NAME_STORAGE_KEY) ?? "";
}

function loadCachedNamePromptSent() {
    if (typeof window === "undefined") {
        return false;
    }
    return window.localStorage.getItem(ONBOARDING_NAME_SENT_STORAGE_KEY) === "true" || Boolean(loadCachedOnboardingName());
}

function normalizeForReport(value: unknown) {
    return JSON.stringify(value, null, 2);
}

function summarizeMemoryDiff(previous: string, next: string) {
    const previousLines = previous.split("\n");
    const nextLines = next.split("\n");
    const maxLines = Math.max(previousLines.length, nextLines.length);
    let changedLines = 0;
    for (let index = 0; index < maxLines; index += 1) {
        if ((previousLines[index] ?? "") !== (nextLines[index] ?? "")) {
            changedLines += 1;
        }
    }
    return `${changedLines} line(s) changed, ${previous.length} -> ${next.length} chars`;
}

function describeSnapshotChange(previous: Snapshot | null, next: Snapshot) {
    const changes: string[] = [];
    const previousState = previous?.runtimeState;
    const nextState = next.runtimeState;
    if (!previousState && nextState) {
        changes.push(`Runtime state observed: ${nextState.state} from ${nextState.sourceTool ?? "unknown"}`);
    } else if (previousState && !nextState) {
        changes.push(`Runtime state cleared from ${previousState.state}`);
    } else if (previousState && nextState) {
        if (previousState.state !== nextState.state) {
            changes.push(`Runtime state: ${previousState.state} -> ${nextState.state}`);
        }
        if (previousState.sourceTool !== nextState.sourceTool) {
            changes.push(`State source: ${previousState.sourceTool ?? "none"} -> ${nextState.sourceTool ?? "none"}`);
        }
        if (previousState.metadata !== nextState.metadata) {
            changes.push(`State metadata changed: ${previousState.metadata} -> ${nextState.metadata}`);
        }
    }

    return changes;
}

function reportWindowStartIso(now = new Date()) {
    return new Date(now.getTime() - REPORT_WINDOW_MS).toISOString();
}

function runtimeSnapshotForReport(value: Snapshot | null) {
    if (!value) {
        return null;
    }
    return {
        ok: value.ok,
        userId: value.userId,
        deviceId: value.deviceId,
        runtimeState: value.runtimeState
    };
}

function diagnosticsForReport(value: ContextReport | null) {
    if (!value) {
        return null;
    }
    return {
        ok: value.ok,
        userId: value.userId,
        provider: value.report.provider,
        model: value.report.model,
        systemPromptChars: value.report.systemPromptChars,
        toolCount: value.report.toolCount,
        memory: value.report.memory,
        runtimeState: value.runtimeState ?? null,
        sleepMetrics: value.sleepMetrics ?? null
    };
}

function reportEventIsRedundant(event: ReportEvent) {
    return [
        "alarms.pending",
        "backend.health",
        "chat.coach",
        "chat.system",
        "chat.user",
        "chat.send",
        "diagnostics.prompt",
        "llm.reply",
        "memory.observed"
    ].includes(event.kind);
}

export default function AntirotLabPage() {
    const [connection, setConnection] = useState<Status>("idle");
    const [testMode, setTestMode] = useState<Status>("idle");
    const [busy, setBusy] = useState(false);
    const [recording, setRecording] = useState(false);
    const [autoSpeak, setAutoSpeak] = useState(true);
    const [messages, setMessages] = useState<ChatMessage[]>(() => [...initialMessages]);
    const [draft, setDraft] = useState("");
    const [onboardingName, setOnboardingName] = useState("");
    const [namePromptSent, setNamePromptSent] = useState(false);
    const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
    const [pendingAlarms, setPendingAlarms] = useState<PendingAlarm[]>([]);
    const [memoryKey, setMemoryKey] = useState("tasks");
    const [memoryContent, setMemoryContent] = useState("Memory will load after the backend connects.");
    const [diagnostics, setDiagnostics] = useState<ContextReport | null>(null);
    const [speechStatus, setSpeechStatus] = useState("VAD speech chunks ready.");
    const [lastError, setLastError] = useState("");
    const [googleStatus, setGoogleStatus] = useState("Google web sign-in not loaded.");
    const [googleResult, setGoogleResult] = useState("Use this to compare browser Google login with the iOS app.");
    const [reportStatus, setReportStatus] = useState("Report captures the last 30 minutes.");
    const [isReporting, setIsReporting] = useState(false);
    const [iosClock, setIosClock] = useState("");
    const [browserReady, setBrowserReady] = useState(false);
    const googleButtonRef = useRef<HTMLDivElement | null>(null);
    const googleButtonRenderedRef = useRef(false);
    const chatLogRef = useRef<HTMLDivElement | null>(null);
    const vadRef = useRef<MicVAD | null>(null);
    const vadBufferRef = useRef<Float32Array[]>([]);
    const vadBufferSecondsRef = useRef(0);
    const vadFlushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const nextSpeechSequenceRef = useRef(0);
    const nextTranscriptSequenceRef = useRef(0);
    const pendingTranscriptResultsRef = useRef(new Map<number, SpeechTranscriptResult>());
    const speechInFlightRef = useRef(0);
    const reportEventsRef = useRef<ReportEvent[]>([]);
    const memorySnapshotsRef = useRef(new Map<string, string>());
    const lastSnapshotRef = useRef<Snapshot | null>(null);

    const stateName = runtimeStateName(snapshot?.runtimeState?.state);
    const stateSource = snapshot?.runtimeState?.sourceTool ?? "none";
    const showNamePrompt = browserReady && (stateName === "onboarding" || stateName === "unknown") && !namePromptSent;

    const labActions = useMemo<LabAction[]>(
        () => [
            {
                id: "start-work",
                label: "Start Work",
                icon: <Play size={16} />,
                tool: "start_session",
                args: { task_id: "Frontend lab validation", estimated_minutes: 25 }
            },
            {
                id: "extend-work",
                label: "Extend",
                icon: <RefreshCw size={16} />,
                tool: "extend_session",
                args: { extension_minutes: 10 }
            },
            {
                id: "done",
                label: "Done",
                icon: <Check size={16} />,
                tool: "end_session",
                args: { actual_minutes: 25, productive_level: 80 }
            },
            {
                id: "break",
                label: "Break",
                icon: <Coffee size={16} />,
                tool: "start_break",
                args: { duration_minutes: 10 }
            },
            {
                id: "wake",
                label: "Awake",
                icon: <Moon size={16} />,
                tool: "log_wake",
                args: { sleep_quality: 4 }
            },
        ],
        []
    );
    const visibleQuickMessages = useMemo(
        () => quickMessages.filter((message) => quickMessagesByState[stateName].includes(message.id)),
        [stateName]
    );
    const visibleLabActions = useMemo(
        () => labActions.filter((action) => actionsByState[stateName].includes(action.id)),
        [labActions, stateName]
    );
    const visiblePendingAlarms = useMemo(
        () => collapsePendingAlarmsToNextReminder(pendingAlarms),
        [pendingAlarms]
    );

    useEffect(() => {
        void bootLab();
        return () => {
            void cleanupVad();
        };
    }, []);

    useEffect(() => {
        setOnboardingName(loadCachedOnboardingName());
        setNamePromptSent(loadCachedNamePromptSent());
        setBrowserReady(true);
    }, []);

    useEffect(() => {
        if (!browserReady || googleButtonRenderedRef.current) {
            return;
        }

        if (!GOOGLE_WEB_CLIENT_ID) {
            setGoogleStatus("Google web client ID missing.");
            setGoogleResult(
                "Create a Google OAuth client of type Web application, add http://localhost:3000 to Authorized JavaScript origins, then set GOOGLE_WEB_CLIENT_ID and NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID."
            );
            return;
        }

        function renderGoogleButton() {
            if (!googleButtonRef.current || !window.google || googleButtonRenderedRef.current) {
                return;
            }

            window.google.accounts.id.initialize({
                client_id: GOOGLE_WEB_CLIENT_ID,
                callback: (response) => void handleGoogleCredential(response),
                ux_mode: "popup"
            });
            window.google.accounts.id.renderButton(googleButtonRef.current, {
                theme: "outline",
                size: "large",
                type: "standard",
                shape: "pill",
                text: "signin_with",
                width: 260
            });
            googleButtonRenderedRef.current = true;
            setGoogleStatus("Google button ready.");
        }

        if (window.google) {
            renderGoogleButton();
            return;
        }

        const existingScript = document.querySelector<HTMLScriptElement>("script[data-antirot-google-signin]");
        if (existingScript) {
            existingScript.addEventListener("load", renderGoogleButton, { once: true });
            return () => existingScript.removeEventListener("load", renderGoogleButton);
        }

        const script = document.createElement("script");
        script.src = "https://accounts.google.com/gsi/client";
        script.async = true;
        script.defer = true;
        script.dataset.antirotGoogleSignin = "true";
        script.addEventListener("load", renderGoogleButton, { once: true });
        script.addEventListener("error", () => {
            setGoogleStatus("Google script failed to load.");
            setGoogleResult("The browser could not load https://accounts.google.com/gsi/client.");
        }, { once: true });
        document.head.appendChild(script);

        return () => script.removeEventListener("load", renderGoogleButton);
    }, [browserReady]);

    useEffect(() => {
        function updateClock() {
            setIosClock(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }));
        }
        updateClock();
        const timer = window.setInterval(updateClock, 30_000);
        return () => window.clearInterval(timer);
    }, []);

    useEffect(() => {
        if (connection === "ok") {
            void loadMemory(memoryKey);
        }
    }, [connection, memoryKey]);

    useEffect(() => {
        const chatLog = chatLogRef.current;
        if (!chatLog) {
            return;
        }
        chatLog.scrollTop = chatLog.scrollHeight;
    }, [messages]);

    async function handleGoogleCredential(response: GoogleCredentialResponse) {
        const credential = response.credential?.trim();
        if (!credential) {
            setGoogleStatus("Google returned no credential.");
            setGoogleResult(JSON.stringify(response, null, 2));
            recordEvent("auth.google.empty", "Google returned no credential.", normalizeForReport(response));
            return;
        }

        setGoogleStatus("Google token received. Posting to Antirot backend...");
        setGoogleResult(`Credential JWT received (${credential.length} chars).`);
        recordEvent("auth.google.token", `Google credential received (${credential.length} chars).`);

        try {
            const result = await backendJson<unknown>(
                "/v1/auth/google",
                {
                    method: "POST",
                    body: JSON.stringify({
                        idToken: credential,
                        deviceId: `frontend-google-${crypto.randomUUID()}`,
                        platform: "web",
                        appVersion: "frontend-lab",
                        notificationCapability: "browser_test",
                        usageCapability: "none"
                    })
                },
                ""
            );
            setGoogleStatus("Backend Google login succeeded.");
            setGoogleResult(JSON.stringify(result, null, 2));
            recordEvent("auth.google.success", "Backend Google login succeeded.", normalizeForReport(result));
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            setGoogleStatus("Backend Google login failed.");
            setGoogleResult(message);
            recordEvent("auth.google.fail", `Backend Google login failed: ${message}`);
        }
    }

    function recordEvent(kind: string, summary: string, detail?: string) {
        const now = new Date();
        const cutoff = now.getTime() - REPORT_WINDOW_MS;
        const nextEvent: ReportEvent = {
            id: `${now.getTime()}-${Math.random().toString(16).slice(2)}`,
            at: now.toISOString(),
            kind,
            summary,
            ...(detail ? { detail } : {})
        };
        reportEventsRef.current = [
            ...reportEventsRef.current.filter((event) => new Date(event.at).getTime() >= cutoff),
            nextEvent
        ];
    }

    function trackSnapshotChange(source: string, next: Snapshot) {
        const changes = describeSnapshotChange(lastSnapshotRef.current, next);
        if (changes.length > 0) {
            recordEvent("state.snapshot", `${source}: ${changes.length} state change(s)`, changes.join("\n\n"));
        }
        lastSnapshotRef.current = next;
    }

    function trackMemoryObservation(key: string, content: string, source: string) {
        const previous = memorySnapshotsRef.current.get(key);
        if (previous !== undefined && previous !== content) {
            recordEvent(
                "memory.changed",
                `${source}: ${key}.md changed (${summarizeMemoryDiff(previous, content)})`,
                [`Before:\n${previous}`, `After:\n${content}`].join("\n\n---\n\n")
            );
        }
        memorySnapshotsRef.current.set(key, content);
    }

    function pushMessage(role: Role, text: string, extras: Partial<Pick<ChatMessage, "audioUrl" | "audioSeconds">> = {}) {
        recordEvent(
            `chat.${role}`,
            `${role === "coach" ? "Antirot" : role} message${extras.audioSeconds ? ` (${extras.audioSeconds.toFixed(1)}s audio)` : ""}`,
            text
        );
        setMessages((current) => [
            ...current,
            {
                id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
                role,
                text,
                at: nowLabel(),
                ...extras
            }
        ]);
    }

    function removeMessage(id: string) {
        setMessages((current) => current.filter((message) => message.id !== id));
    }

    async function resetBrowserConversation() {
        recordEvent("button.resetConversation", "Reset browser conversation pressed.");
        window.localStorage.removeItem(ONBOARDING_NAME_STORAGE_KEY);
        window.localStorage.removeItem(ONBOARDING_NAME_SENT_STORAGE_KEY);
        setMessages([...initialMessages]);
        setDraft("");
        setLastError("");
        setOnboardingName("");
        setNamePromptSent(false);
        setSpeechStatus("VAD speech chunks ready.");
        setMemoryContent("Resetting backend fixture and memory files...");
        pendingTranscriptResultsRef.current.clear();
        nextTranscriptSequenceRef.current = nextSpeechSequenceRef.current;

        setBusy(true);
        try {
            const reset = await backendJson<Snapshot>("/v1/test/reset", {
                method: "POST",
                body: JSON.stringify({
                    userId: USER_ID,
                    deviceId: DEVICE_ID,
                    deviceToken: DEVICE_TOKEN
                })
            });
            setSnapshot(reset);
            trackSnapshotChange("backend fixture reset", reset);
            setTestMode("ok");
            setMessages([
                ...initialMessages,
                {
                    id: `reset-${Date.now()}`,
                    role: "system",
                    text: "Browser conversation and backend memory files reset.",
                    at: nowLabel()
                }
            ]);
            await refreshAll();
        } catch (error) {
            handleError(error, "Backend memory reset failed");
        } finally {
            setBusy(false);
        }
    }

    function handleError(error: unknown, fallback: string) {
        const message = error instanceof Error ? error.message : fallback;
        setLastError(message);
        recordEvent("error", `${fallback}: ${message}`);
        pushMessage("system", `${fallback}: ${message}`);
    }

    async function bootLab() {
        recordEvent("button.resetLab", "Reset lab / boot flow started.");
        setConnection("loading");
        setTestMode("loading");
        setLastError("");
        try {
            await backendJson<{ ok: boolean }>("/v1/health", { method: "GET" }, "");
            setConnection("ok");
            recordEvent("backend.health", `Backend health online at ${BACKEND_URL}.`);
            pushMessage("system", `Backend health is online at ${BACKEND_URL}.`);
        } catch (error) {
            setConnection("fail");
            setTestMode("fail");
            handleError(error, "Backend health check failed");
            return;
        }

        try {
            const reset = await backendJson<Snapshot>("/v1/test/reset", {
                method: "POST",
                body: JSON.stringify({
                    userId: USER_ID,
                    deviceId: DEVICE_ID,
                    deviceToken: DEVICE_TOKEN
                })
            });
            setSnapshot(reset);
            trackSnapshotChange("boot test reset", reset);
            setTestMode("ok");
            pushMessage("system", "Test fixture reset. Direct state actions are enabled.");
        } catch (error) {
            setTestMode("fail");
            const authHint = ADMIN_TOKEN === "test-admin-token"
                ? " Start the lab with ANTIROT_ADMIN_TOKEN/NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN set."
                : "";
            handleError(error, `Test endpoints are not available.${authHint}`);
        }

        await refreshAll();
    }

    async function refreshAll() {
        await Promise.allSettled([loadSnapshot(), loadPendingAlarms(), loadMemory(memoryKey), loadDiagnostics()]);
    }

    async function loadSnapshot() {
        try {
            const state = await backendJson<Snapshot>(
                `/v1/test/state?userId=${encodeURIComponent(USER_ID)}&deviceId=${encodeURIComponent(DEVICE_ID)}`
            );
            setSnapshot(state);
            trackSnapshotChange("state refresh", state);
        } catch {
            setSnapshot((current) => current);
        }
    }

    async function loadPendingAlarms() {
        try {
            const alarms = await backendJson<PendingAlarm[]>(
                `/v1/alarms/pending?device_id=${encodeURIComponent(DEVICE_ID)}`,
                {},
                DEVICE_TOKEN
            );
            setPendingAlarms(alarms);
        } catch {
            setPendingAlarms([]);
        }
    }

    async function loadMemory(key: string) {
        const targetKey = key === "work" ? todayWorkKey() : key;
        setMemoryKey(key);
        try {
            const memory = await backendJson<MemoryResponse>(`/v1/memory/${encodeURIComponent(targetKey)}`, {}, DEVICE_TOKEN);
            setMemoryContent(memory.content || "# Empty\n");
            trackMemoryObservation(targetKey, memory.content || "# Empty\n", "memory tab load");
        } catch (error) {
            setMemoryContent(error instanceof Error ? `Could not load ${targetKey}: ${error.message}` : `Could not load ${targetKey}.`);
        }
    }

    async function loadDiagnostics() {
        try {
            const report = await backendJson<ContextReport>(
                `/v1/admin/context?userId=${encodeURIComponent(USER_ID)}&provider=gemini&model=gemini-3.5-flash`
            );
            setDiagnostics(report);
            recordEvent("diagnostics.prompt", `Diagnostics loaded: ${report.report.systemPromptChars} prompt chars, ${report.report.toolCount} tools.`);
        } catch {
            setDiagnostics(null);
        }
    }

    async function sendChat(text: string, visibleText?: string) {
        const trimmed = text.trim();
        if (!trimmed || busy) {
            return;
        }
        const visible = visibleText === undefined ? trimmed : visibleText.trim();
        setBusy(true);
        setDraft("");
        if (visible) {
            recordEvent("chat.send", "User sent chat to coach.", visible);
            pushMessage("user", visible);
        } else {
            recordEvent("chat.send.hidden", "Hidden client onboarding message sent to coach.", trimmed);
        }
        try {
            const reply = await backendJson<{ ok: boolean; reply: string }>("/v1/chat", {
                method: "POST",
                body: JSON.stringify({ message: trimmed })
            });
            recordEvent("llm.reply", "Coach LLM reply received.", reply.reply);
            pushMessage("coach", reply.reply);
            if (autoSpeak) {
                void speakText(reply.reply);
            }
            await refreshAll();
        } catch (error) {
            handleError(error, "Coach chat failed");
        } finally {
            setBusy(false);
        }
    }

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        await sendChat(draft);
    }

    async function submitOnboarding(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const name = onboardingName.trim();
        if (!name) {
            return;
        }
        recordEvent("onboarding.name", `Onboarding name submitted: ${name}`);
        window.localStorage.setItem(ONBOARDING_NAME_STORAGE_KEY, name);
        window.localStorage.setItem(ONBOARDING_NAME_SENT_STORAGE_KEY, "true");
        setNamePromptSent(true);
        await sendChat(onboardingMessage(name), "");
    }

    async function runTool(action: LabAction) {
        recordEvent("button.stateAction", `${action.label} pressed.`, normalizeForReport({ tool: action.tool, args: action.args }));
        setBusy(true);
        try {
            const result = await backendJson<{ ok: boolean; result: string; snapshot: Snapshot }>("/v1/test/tool", {
                method: "POST",
                body: JSON.stringify({
                    userId: USER_ID,
                    name: action.tool,
                    args: action.args
                })
            });
            setSnapshot(result.snapshot);
            trackSnapshotChange(`tool ${action.tool}`, result.snapshot);
            recordEvent("tool.result", `${action.label}: ${result.result}`);
            pushMessage("system", `${action.label}: ${result.result}`);
            await refreshAll();
        } catch (error) {
            handleError(error, `${action.label} failed`);
        } finally {
            setBusy(false);
        }
    }

    async function acknowledgeAlarm(alarmId: string, action: "ack" | "dismiss" | "snooze" | "clear") {
        recordEvent("button.alarmAction", `Alarm ${action} pressed for ${alarmId}.`);
        try {
            await backendJson(`/v1/alarms/${encodeURIComponent(alarmId)}/${action}`, {
                method: "POST",
                body: JSON.stringify({
                    deviceId: DEVICE_ID,
                    action,
                    at: new Date().toISOString()
                })
            }, DEVICE_TOKEN);
            await refreshAll();
        } catch (error) {
            handleError(error, "Alarm action failed");
        }
    }

    async function startRecording() {
        recordEvent("button.voiceStart", "Voice capture start pressed.");
        if (!navigator.mediaDevices?.getUserMedia) {
            handleError(new Error("Microphone capture is unavailable in this browser."), "Voice capture failed");
            return;
        }
        try {
            clearVadFlushTimer();
            vadBufferRef.current = [];
            vadBufferSecondsRef.current = 0;
            const { MicVAD: BrowserMicVAD, utils } = await import("@ricky0123/vad-web");
            const vad = await BrowserMicVAD.new({
                model: "v5",
                baseAssetPath: "/vad/",
                onnxWASMBasePath: "/vad/",
                positiveSpeechThreshold: 0.48,
                negativeSpeechThreshold: 0.32,
                preSpeechPadMs: 500,
                redemptionMs: 1800,
                minSpeechMs: 1000,
                submitUserSpeechOnPause: true,
                onSpeechStart: () => {
                    clearVadFlushTimer();
                    setSpeechStatus("Listening: speech detected...");
                    recordEvent("voice.speechStart", "VAD detected speech.");
                },
                onSpeechRealStart: () => {
                    setSpeechStatus("Capturing utterance...");
                    recordEvent("voice.realStart", "VAD confirmed real speech.");
                },
                onVADMisfire: () => {
                    setSpeechStatus("Ignored a very short voice blip.");
                    recordEvent("voice.misfire", "Ignored a very short voice blip.");
                },
                onSpeechEnd: (audio) => {
                    vadBufferRef.current.push(audio);
                    vadBufferSecondsRef.current += audio.length / VAD_SAMPLE_RATE;
                    const bufferedSeconds = vadBufferSecondsRef.current;
                    if (bufferedSeconds >= VAD_HARD_UPLOAD_SECONDS || bufferedSeconds >= VAD_PREFERRED_UPLOAD_SECONDS) {
                        recordEvent("voice.bufferFlush", `Flushing ${bufferedSeconds.toFixed(1)}s voice buffer by threshold.`);
                        void flushVadBuffer(utils.encodeWAV, "vad-threshold");
                    } else if (bufferedSeconds >= VAD_MIN_UPLOAD_SECONDS) {
                        setSpeechStatus(`Buffered ${bufferedSeconds.toFixed(1)}s. Waiting for settled silence...`);
                        scheduleVadFlush(utils.encodeWAV);
                    } else {
                        setSpeechStatus(`Buffered ${bufferedSeconds.toFixed(1)}s. Minimum chunk is ${VAD_MIN_UPLOAD_SECONDS}s.`);
                    }
                }
            });
            vadRef.current = vad;
            await vad.start();
            setRecording(true);
            setSpeechStatus("VAD listening. Speak naturally.");
            recordEvent("voice.started", "VAD voice capture started.");
        } catch (error) {
            await cleanupVad();
            handleError(error, "Voice capture failed");
        }
    }

    async function stopRecording() {
        recordEvent("button.voiceStop", "Voice capture stop pressed.");
        await cleanupVad();
        await flushVadBufferFromPackage("manual-stop");
        if (speechInFlightRef.current === 0) {
            setSpeechStatus("VAD stopped.");
        }
        setRecording(false);
    }

    async function cleanupVad() {
        clearVadFlushTimer();
        if (vadRef.current) {
            await vadRef.current.pause();
            await vadRef.current.destroy();
            vadRef.current = null;
        }
    }

    function clearVadFlushTimer() {
        if (vadFlushTimerRef.current) {
            clearTimeout(vadFlushTimerRef.current);
            vadFlushTimerRef.current = null;
        }
    }

    function scheduleVadFlush(encodeWAV: (samples: Float32Array, format?: number, sampleRate?: number, numChannels?: number, bitDepth?: number) => ArrayBuffer) {
        clearVadFlushTimer();
        vadFlushTimerRef.current = setTimeout(() => {
            void flushVadBuffer(encodeWAV, "settled-silence");
        }, VAD_SETTLED_SILENCE_MS);
    }

    async function flushVadBufferFromPackage(reason: string) {
        if (!vadBufferRef.current.length) {
            return;
        }
        const { utils } = await import("@ricky0123/vad-web");
        await flushVadBuffer(utils.encodeWAV, reason);
    }

    async function flushVadBuffer(
        encodeWAV: (samples: Float32Array, format?: number, sampleRate?: number, numChannels?: number, bitDepth?: number) => ArrayBuffer,
        reason: string
    ) {
        clearVadFlushTimer();
        const chunks = vadBufferRef.current;
        const seconds = vadBufferSecondsRef.current;
        if (!chunks.length) {
            return;
        }
        vadBufferRef.current = [];
        vadBufferSecondsRef.current = 0;
        const audio = concatFloat32(chunks);
        const wav = encodeWAV(audio, 1, VAD_SAMPLE_RATE, 1, 16);
        const blob = new Blob([wav], { type: "audio/wav" });
        recordEvent("voice.chunk", `Captured ${seconds.toFixed(1)}s voice chunk.`, `reason=${reason}, samples=${audio.length}`);
        pushMessage("user", "Voice message", {
            audioUrl: URL.createObjectURL(blob),
            audioSeconds: seconds
        });
        startSpeechTranscription({ blob, seconds, reason, sequence: nextSpeechSequenceRef.current });
        nextSpeechSequenceRef.current += 1;
    }

    function startSpeechTranscription(item: SpeechChunkItem) {
        speechInFlightRef.current += 1;
        setSpeechStatus(`Transcribing ${speechInFlightRef.current} speech chunk(s)...`);
        void transcribeVadBlob(item);
    }

    function updateSpeechStatus() {
        const inFlight = speechInFlightRef.current;
        const waiting = pendingTranscriptResultsRef.current.size;
        if (inFlight > 0 && waiting > 0) {
            setSpeechStatus(`Transcribing ${inFlight} chunk(s). ${waiting} result(s) waiting for earlier audio.`);
        } else if (inFlight > 0) {
            setSpeechStatus(`Transcribing ${inFlight} speech chunk(s)...`);
        } else if (waiting > 0) {
            setSpeechStatus(`${waiting} speech result(s) waiting for earlier audio.`);
        } else {
            setSpeechStatus("Speech chunks transcribed.");
        }
    }

    function flushOrderedSpeechResults() {
        while (pendingTranscriptResultsRef.current.has(nextTranscriptSequenceRef.current)) {
            const result = pendingTranscriptResultsRef.current.get(nextTranscriptSequenceRef.current);
            pendingTranscriptResultsRef.current.delete(nextTranscriptSequenceRef.current);
            nextTranscriptSequenceRef.current += 1;
            if (!result) {
                continue;
            }
            if (result.error) {
                recordEvent("speech.transcribe.fail", `Speech transcription failed for ${result.seconds.toFixed(1)}s chunk.`, result.error);
                pushMessage("system", `Speech-to-text failed: ${result.error}`);
                continue;
            }
            if (!result.text) {
                recordEvent("speech.transcribe.empty", `No transcript returned for ${result.seconds.toFixed(1)}s voice chunk.`);
                pushMessage("system", `No transcript returned for ${result.seconds.toFixed(1)}s voice chunk.`);
                continue;
            }
            recordEvent("speech.transcribe.success", `Transcript returned for ${result.seconds.toFixed(1)}s voice chunk.`, result.text);
            setDraft((current) => (current.trim() ? `${current.trim()} ${result.text}` : result.text));
        }
        updateSpeechStatus();
    }

    async function transcribeVadBlob({ blob, seconds, reason, sequence }: SpeechChunkItem) {
        try {
            const form = new FormData();
            form.append("file", blob, "voice-segment.wav");
            const response = await backendJson<{ ok: boolean; text: string }>("/v1/speech/transcribe", {
                method: "POST",
                body: form
            });
            pendingTranscriptResultsRef.current.set(sequence, {
                text: response.text.trim(),
                seconds,
                reason
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : "Speech-to-text failed";
            setLastError(message);
            pendingTranscriptResultsRef.current.set(sequence, {
                text: "",
                seconds,
                reason,
                error: message
            });
        } finally {
            speechInFlightRef.current = Math.max(0, speechInFlightRef.current - 1);
            flushOrderedSpeechResults();
        }
    }

    async function speakText(text: string) {
        const clean = text.trim();
        if (!clean) {
            return;
        }
        try {
            const response = await backendJson<{ ok: boolean; audioBase64: string; contentType?: string }>("/v1/speech/synthesize", {
                method: "POST",
                body: JSON.stringify({ text: clean.slice(0, 1200) })
            });
            const audio = new Audio(`data:${response.contentType ?? "audio/mpeg"};base64,${response.audioBase64}`);
            await audio.play();
            recordEvent("speech.synthesize.success", `TTS playback started (${response.contentType ?? "audio/mpeg"}).`);
        } catch (error) {
            handleError(error, "Text-to-speech failed");
        }
    }

    async function loadReportMemorySnapshots() {
        const rows: ReportMemorySnapshot[] = [];
        for (const tab of memoryTabs) {
            const key = tab.key === "work" ? todayWorkKey() : tab.key;
            try {
                const memory = await backendJson<MemoryResponse>(`/v1/memory/${encodeURIComponent(key)}`, {}, DEVICE_TOKEN);
                const content = memory.content || "# Empty\n";
                const previous = memorySnapshotsRef.current.get(key);
                const summary = previous === undefined
                    ? "No earlier browser-session baseline was observed; current content included."
                    : previous === content
                        ? "No change since the frontend last observed this file."
                        : summarizeMemoryDiff(previous, content);
                rows.push({
                    key,
                    content,
                    ...(previous !== undefined ? { previous } : {}),
                    summary
                });
                trackMemoryObservation(key, content, "report capture");
            } catch (error) {
                rows.push({
                    key,
                    content: error instanceof Error ? `Could not load ${key}: ${error.message}` : `Could not load ${key}.`,
                    summary: "Load failed during report capture."
                });
            }
        }
        return rows;
    }

    function buildReportMarkdown(events: ReportEvent[], memoryRows: ReportMemorySnapshot[], now = new Date()) {
        const windowStart = reportWindowStartIso(now);
        const reportCreated = now.toISOString();
        const timelineEvents = events.filter((event) => !reportEventIsRedundant(event));
        const eventLines = timelineEvents.length
            ? timelineEvents.map((event, index) => [
                `### ${index + 1}. ${event.kind} @ ${event.at}`,
                event.summary,
                event.detail ? `\n\`\`\`text\n${event.detail}\n\`\`\`` : ""
            ].filter(Boolean).join("\n")).join("\n\n")
            : "No non-chat state/action/error events were recorded in this 30-minute window.";

        const chatLines = messages.length
            ? messages.map((message, index) => [
                `### ${index + 1}. ${message.role} / ${message.at}`,
                message.audioUrl
                    ? `Voice message${message.audioSeconds ? ` (${message.audioSeconds.toFixed(1)}s)` : ""}`
                    : message.text
            ].join("\n")).join("\n\n")
            : "No browser-visible chat messages.";

        const changedMemoryRows = memoryRows.filter((row) => row.previous !== undefined && row.previous !== row.content);
        const failedMemoryRows = memoryRows.filter((row) => row.previous === undefined && row.summary === "Load failed during report capture.");
        const memoryLines = [...changedMemoryRows, ...failedMemoryRows].map((row) => [
            `### ${row.key}.md`,
            `Summary: ${row.summary}`,
            row.previous !== undefined && row.previous !== row.content
                ? `\nBefore:\n\`\`\`md\n${row.previous}\n\`\`\`\n\nAfter:\n\`\`\`md\n${row.content}\n\`\`\``
                : `\n\`\`\`text\n${row.content}\n\`\`\``
        ].join("\n")).join("\n\n") || "No memory file changed after this frontend session started.";
        const observedMemoryKeys = memoryRows.map((row) => `${row.key}.md`).join(", ") || "none";

        return [
            "# Antirot Frontend Flow Report",
            "",
            `Created: ${reportCreated}`,
            `Window: ${windowStart} -> ${reportCreated}`,
            `Backend: ${BACKEND_URL}`,
            `User ID: ${USER_ID}`,
            `Device ID: ${DEVICE_ID}`,
            `Observed memory files: ${observedMemoryKeys}`,
            "",
            "## Current Runtime State",
            "```json",
            normalizeForReport(runtimeSnapshotForReport(snapshot)),
            "```",
            "",
            "## Backend Diagnostics Summary",
            "```json",
            normalizeForReport(diagnosticsForReport(diagnostics)),
            "```",
            "",
            "## Browser Chat Messages",
            chatLines,
            "",
            "## Memory File Observations",
            memoryLines,
            "",
            "## Detailed Event Timeline",
            eventLines
        ].join("\n");
    }

    async function copyTextToClipboard(text: string) {
        if (navigator.clipboard?.writeText) {
            await navigator.clipboard.writeText(text);
            return;
        }
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.setAttribute("readonly", "true");
        textarea.style.position = "fixed";
        textarea.style.left = "-9999px";
        textarea.style.top = "0";
        document.body.appendChild(textarea);
        textarea.focus();
        textarea.select();
        try {
            if (!document.execCommand("copy")) {
                throw new Error("Browser rejected the fallback clipboard copy.");
            }
        } finally {
            document.body.removeChild(textarea);
        }
    }

    async function createReport() {
        if (isReporting) {
            return;
        }
        const startedAt = new Date();
        recordEvent("button.report", "Report button pressed.");
        setIsReporting(true);
        setReportStatus("Building report...");
        try {
            const windowStart = reportWindowStartIso(startedAt);
            const memoryRows = await loadReportMemorySnapshots();
            const windowEvents = reportEventsRef.current.filter((event) => new Date(event.at).getTime() >= startedAt.getTime() - REPORT_WINDOW_MS);
            const reportMarkdown = buildReportMarkdown(windowEvents, memoryRows, startedAt);
            setReportStatus("Saving report to backend...");
            const saved = await backendJson<CreateReportResponse>("/v1/reports", {
                method: "POST",
                body: JSON.stringify({
                    deviceId: DEVICE_ID,
                    title: "Frontend lab flow report",
                    windowStart,
                    windowEnd: startedAt.toISOString(),
                    reportMarkdown,
                    events: windowEvents.map(({ at, kind, summary, detail }) => ({
                        at,
                        kind,
                        summary,
                        detail
                    }))
                })
            }, DEVICE_TOKEN);

            setReportStatus("Copying report to clipboard...");
            try {
                await copyTextToClipboard(reportMarkdown);
                recordEvent("report.saved", `Report saved and copied. id=${saved.reportId}`);
                setReportStatus(`Report copied and saved: ${saved.reportId}`);
                pushMessage("system", `Report copied to clipboard and saved as ${saved.reportId}.`);
            } catch (clipboardError) {
                const copyMessage = clipboardError instanceof Error ? clipboardError.message : "Clipboard copy failed.";
                recordEvent("report.copyFailed", `Report saved but clipboard copy failed. id=${saved.reportId}`, copyMessage);
                setReportStatus(`Report saved as ${saved.reportId}, but clipboard copy failed.`);
                pushMessage("system", `Report saved as ${saved.reportId}, but clipboard copy failed: ${copyMessage}`);
            }
        } catch (error) {
            const message = error instanceof Error ? error.message : "Report failed";
            recordEvent("report.failed", `Report failed: ${message}`);
            setReportStatus(`Report failed: ${message}`);
            handleError(error, "Report failed");
        } finally {
            setIsReporting(false);
        }
    }

    const latestCoachText = [...messages].reverse().find((message) => message.role === "coach")?.text ?? "";

    return (
        <main className="phone-stage">
            {showNamePrompt ? (
                <div className="name-modal-backdrop">
                    <form className="name-modal" onSubmit={(event) => void submitOnboarding(event)}>
                        <PanelHeader icon={<Brain size={18} />} title="Your name" />
                        <div className="name-modal-body">
                            <input
                                autoFocus
                                value={onboardingName}
                                onChange={(event) => setOnboardingName(event.target.value)}
                                placeholder="Name"
                            />
                            <button type="submit" disabled={busy || !onboardingName.trim()}>
                                {busy ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
                                Continue
                            </button>
                        </div>
                    </form>
                </div>
            ) : null}
            <div className="iphone-frame" aria-label="iPhone preview of Antirot Lab">
                <div className="iphone-side-button left" aria-hidden="true" />
                <div className="iphone-side-button right top" aria-hidden="true" />
                <div className="iphone-side-button right bottom" aria-hidden="true" />
                <div className="iphone-screen">
                    <div className="dynamic-island" aria-hidden="true" />
                    <div className="ios-statusbar" aria-hidden="true">
                        <span>{iosClock}</span>
                        <span className="ios-sensors">
                            <span className="ios-signal" />
                            <span>5G</span>
                            <span className="ios-battery" />
                        </span>
                    </div>
                    <div className="lab-shell">
                        <section className="topbar">
                            <div>
                                <p className="eyebrow">Antirot Lab</p>
                                <h1>Coach simulator</h1>
                            </div>
                            <div className="status-row">
                                <StatusPill label="Backend" status={connection} />
                                <StatusPill label="Test fixture" status={testMode} />
                                <button className="icon-button" type="button" onClick={() => void bootLab()} aria-label="Reset lab">
                                    <RefreshCw size={18} />
                                </button>
                            </div>
                        </section>

                        <section className="hero-grid">
                            <article className="voice-orb-panel">
                                <div className="orb-wrap" aria-hidden="true">
                                    <svg viewBox="0 0 160 160" className={`focus-dial${recording ? " recording" : ""}`}>
                                        <defs>
                                            <radialGradient id="dialCenterGradient" cx="50%" cy="50%" r="50%">
                                                <stop offset="0%" stopColor="#e11d48" stopOpacity="0.3" />
                                                <stop offset="100%" stopColor="#08070b" stopOpacity="1" />
                                            </radialGradient>
                                        </defs>
                                        <circle className="dial-ring-outer" cx="80" cy="80" r="72" />
                                        <circle className="dial-ring-mid" cx="80" cy="80" r="56" />
                                        <circle className="dial-ring-inner" cx="80" cy="80" r="40" />
                                        <circle className="dial-center" cx="80" cy="80" r="24" />
                                        <text x="80" y="80" textAnchor="middle" dominantBaseline="central" fill="#fb7185" fontSize="16" fontWeight="700">{recording ? "◉" : "⚡"}</text>
                                    </svg>
                                </div>
                                <div>
                                    <p className="eyebrow">Voice-first test surface</p>
                                    <h2>{stateName === "unknown" ? "Connect the backend" : `State: ${stateName}`}</h2>
                                    <p className="muted">Speak first, type only when needed. This page tests the same backend paths the apps rely on.</p>
                                </div>
                                <div className="voice-controls">
                                    <button className="primary-button" type="button" onClick={() => void (recording ? stopRecording() : startRecording())}>
                                        {recording ? <Square size={18} /> : <Mic size={18} />}
                                        {recording ? "Stop" : "Speak"}
                                    </button>
                                    <button className="ghost-button" type="button" disabled={!latestCoachText} onClick={() => void speakText(latestCoachText)}>
                                        <Volume2 size={18} />
                                        Speak reply
                                    </button>
                                </div>
                                <label className="toggle">
                                    <input checked={autoSpeak} onChange={(event) => setAutoSpeak(event.target.checked)} type="checkbox" />
                                    Auto-play coach replies
                                </label>
                                <p className="hint">{speechStatus}</p>
                            </article>

                            <article className="state-panel">
                                <PanelHeader icon={<Gauge size={18} />} title="Runtime truth" />
                                <div className="metric-tile">
                                    <span className="metric-label">Current state</span>
                                    <span className="metric-value">{stateName}</span>
                                </div>
                                <div className="metric-tile">
                                    <span className="metric-label">Source</span>
                                    <span className="metric-value">{stateSource}</span>
                                </div>
                                <div className="metric-tile">
                                    <span className="metric-label">Next reminders</span>
                                    <span className="metric-value">{visiblePendingAlarms.length}</span>
                                </div>
                                <div className="metric-tile">
                                    <span className="metric-label">Prompt size</span>
                                    <span className="metric-value">{diagnostics?.report.systemPromptChars ?? "-"}</span>
                                </div>
                            </article>
                        </section>

                        <section className="main-grid">
                            <article className="chat-panel">
                                <PanelHeader
                                    icon={<Brain size={18} />}
                                    title="Coach conversation"
                                    action={
                                        <div className="panel-actions">
                                            <button
                                                aria-label="Copy and save flow report"
                                                className="icon-button panel-action"
                                                disabled={isReporting}
                                                onClick={() => void createReport()}
                                                title="Copy and save flow report"
                                                type="button"
                                            >
                                                <ClipboardList size={17} />
                                            </button>
                                            <button
                                                aria-label="Reset browser conversation"
                                                className="icon-button panel-action"
                                                onClick={() => void resetBrowserConversation()}
                                                title="Reset browser conversation"
                                                type="button"
                                            >
                                                <Trash2 size={17} />
                                            </button>
                                        </div>
                                    }
                                />
                                <p className="hint report-status">{reportStatus}</p>
                                <div className="chat-log" ref={chatLogRef}>
                                    {messages.map((message) => (
                                        <div className={`message ${message.role}`} key={message.id}>
                                            <button
                                                aria-label="Remove message"
                                                className="message-close"
                                                onClick={() => removeMessage(message.id)}
                                                type="button"
                                            >
                                                ×
                                            </button>
                                            {message.audioUrl ? (
                                                <div className="voice-message">
                                                    <audio controls src={message.audioUrl} />
                                                    <span>{message.audioSeconds ? `${message.audioSeconds.toFixed(1)}s` : "voice"}</span>
                                                </div>
                                            ) : (
                                                <p>{message.text}</p>
                                            )}
                                            <span>{message.role === "coach" ? "Antirot" : message.role} / {message.at}</span>
                                        </div>
                                    ))}
                                </div>
                                <div className="quick-grid">
                                    {visibleQuickMessages.map((message) => (
                                        <button key={message.id} type="button" onClick={() => void sendChat(message.text)} disabled={busy}>
                                            {message.label}
                                        </button>
                                    ))}
                                </div>
                                <form className="composer" onSubmit={(event) => void handleSubmit(event)}>
                                    <button
                                        aria-label={recording ? "Stop voice input" : "Start voice input"}
                                        className="composer-speak"
                                        onClick={() => void (recording ? stopRecording() : startRecording())}
                                        type="button"
                                    >
                                        {recording ? <Square size={18} /> : <Mic size={18} />}
                                        {recording ? "Stop" : "Speak"}
                                    </button>
                                    <input
                                        value={draft}
                                        onChange={(event) => setDraft(event.target.value)}
                                        placeholder="Speak or type the user's next message"
                                    />
                                    <button className={`composer-send${draft.trim() ? " has-text" : ""}`} type="submit" disabled={busy || !draft.trim()}>
                                        {busy ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
                                        Send
                                    </button>
                                </form>
                            </article>

                            <aside className="side-stack">
                                <article className="panel">
                                    <PanelHeader icon={<Activity size={18} />} title="Direct state actions" />
                                    <div className="action-grid">
                                        {visibleLabActions.map((action) => (
                                            <button key={action.id} type="button" disabled={busy || testMode !== "ok"} onClick={() => void runTool(action)}>
                                                {action.icon}
                                                {action.label}
                                            </button>
                                        ))}
                                        {visibleLabActions.length === 0 ? (
                                            <p className="empty">No direct actions for this state.</p>
                                        ) : null}
                                    </div>
                                </article>

                                <article className="panel">
                                    <PanelHeader icon={<AlarmClock size={18} />} title="Pending alarms" />
                                    <div className="alarm-list">
                                        {visiblePendingAlarms.length === 0 ? (
                                            <p className="empty">No pending alarms.</p>
                                        ) : (
                                            visiblePendingAlarms.map((alarm) => (
                                                <div className="alarm-card" key={alarm.id}>
                                                    <div>
                                                        <strong>{alarm.title ?? alarm.kind}</strong>
                                                        <span>{alarm.severity} / {formatAlarmTime(alarm)}</span>
                                                    </div>
                                                    <p>{alarm.message ?? "No message."}</p>
                                                    <div className="alarm-buttons">
                                                        <button type="button" onClick={() => void acknowledgeAlarm(alarm.id, "ack")}>Ack</button>
                                                        <button type="button" onClick={() => void acknowledgeAlarm(alarm.id, "snooze")}>Snooze</button>
                                                        <button type="button" onClick={() => void acknowledgeAlarm(alarm.id, "clear")}>Clear</button>
                                                    </div>
                                                </div>
                                            ))
                                        )}
                                    </div>
                                </article>
                            </aside>
                        </section>

                        <section className="lower-grid">
                            <article className="panel memory-panel">
                                <PanelHeader icon={<ClipboardList size={18} />} title="Memory logs" />
                                <div className="tabs">
                                    {memoryTabs.map((tab) => (
                                        <button
                                            className={memoryKey === tab.key ? "active" : ""}
                                            key={tab.key}
                                            type="button"
                                            onClick={() => void loadMemory(tab.key)}
                                        >
                                            {tab.label}
                                        </button>
                                    ))}
                                </div>
                                <pre>{memoryContent}</pre>
                            </article>

                            <article className="panel diagnostics-panel">
                                <PanelHeader icon={<HeartPulse size={18} />} title="Diagnostics" />
                                <dl>
                                    <dt>Backend URL</dt>
                                    <dd>{BACKEND_URL}</dd>
                                    <dt>User / device</dt>
                                    <dd>{USER_ID} / {DEVICE_ID}</dd>
                                    <dt>Provider</dt>
                                    <dd>{diagnostics ? `${diagnostics.report.provider} / ${diagnostics.report.model}` : "Unavailable"}</dd>
                                    <dt>Tools</dt>
                                    <dd>{diagnostics?.report.toolCount ?? "-"}</dd>
                                    <dt>Memory budget</dt>
                                    <dd>
                                        {diagnostics
                                            ? `${diagnostics.report.memory.totalInjectedChars} / ${diagnostics.report.memory.totalMemoryBudgetChars}`
                                            : "-"}
                                    </dd>
                                    <dt>Truncated sections</dt>
                                    <dd>{diagnostics?.report.memory.truncatedSections.join(", ") || "None"}</dd>
                                    <dt>Sleep samples</dt>
                                    <dd>{diagnostics?.sleepMetrics?.sleepSampleCount ?? "-"}</dd>
                                </dl>
                                <div className="google-login-test">
                                    <div>
                                        <h3>Google Login Test</h3>
                                        <p>Client: {GOOGLE_WEB_CLIENT_ID || "missing NEXT_PUBLIC_GOOGLE_WEB_CLIENT_ID"}</p>
                                    </div>
                                    <div ref={googleButtonRef} className="google-button-slot" />
                                    <strong>{googleStatus}</strong>
                                    <pre>{googleResult}</pre>
                                </div>
                                {lastError ? <p className="error-box">{lastError}</p> : null}
                            </article>
                        </section>
                    </div>
                </div>
            </div>
        </main>
    );
}

function StatusPill({ label, status }: { label: string; status: Status }) {
    return (
        <span className={`status-pill ${status}`}>
            <span />
            {label}: {status}
        </span>
    );
}

function PanelHeader({ icon, title, action }: { icon: React.ReactNode; title: string; action?: React.ReactNode }) {
    return (
        <div className="panel-header">
            <div>
                <span className="panel-icon">{icon}</span>
                <h2>{title}</h2>
            </div>
            {action}
        </div>
    );
}

function collapsePendingAlarmsToNextReminder(alarms: PendingAlarm[]) {
    const nextByReminder = new Map<string, PendingAlarm>();
    for (const alarm of alarms) {
        const key = alarmReminderKey(alarm);
        const current = nextByReminder.get(key);
        if (!current || alarmFireTime(alarm) < alarmFireTime(current)) {
            nextByReminder.set(key, alarm);
        }
    }

    return Array.from(nextByReminder.values()).sort((a, b) => alarmFireTime(a) - alarmFireTime(b));
}

function alarmReminderKey(alarm: PendingAlarm) {
    return [
        alarm.kind,
        alarm.title ?? "",
        alarm.message ?? ""
    ].join("|");
}

function alarmFireTime(alarm: PendingAlarm) {
    const raw = alarm.fire_at ?? alarm.fireAt;
    if (!raw) {
        return Number.MAX_SAFE_INTEGER;
    }
    const value = new Date(raw).getTime();
    return Number.isNaN(value) ? Number.MAX_SAFE_INTEGER : value;
}

function formatAlarmTime(alarm: PendingAlarm) {
    const raw = alarm.fire_at ?? alarm.fireAt;
    if (!raw) {
        return "unscheduled";
    }
    return new Date(raw).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit"
    });
}
