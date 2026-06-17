"use client";

import {
    Activity,
    AlarmClock,
    Bed,
    Brain,
    Check,
    ClipboardList,
    Coffee,
    Gauge,
    HeartPulse,
    Loader2,
    Mic,
    Moon,
    Plane,
    Play,
    RefreshCw,
    Send,
    Square,
    Volume2
} from "lucide-react";
import type { FormEvent } from "react";
import { useEffect, useMemo, useRef, useState } from "react";

const BACKEND_URL = process.env.NEXT_PUBLIC_ANTIROT_BACKEND_URL || "https://api.antirot.org";
const ADMIN_TOKEN = process.env.NEXT_PUBLIC_ANTIROT_ADMIN_TOKEN || "test-admin-token";
const DEVICE_TOKEN = process.env.NEXT_PUBLIC_ANTIROT_DEVICE_TOKEN || "test-device-token";
const USER_ID = "admin";
const DEVICE_ID = "frontend-lab-device";

type Role = "user" | "coach" | "system";
type Status = "idle" | "loading" | "ok" | "fail";

type ChatMessage = {
    id: string;
    role: Role;
    text: string;
    at: string;
};

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

type LabAction = {
    id: string;
    label: string;
    icon: React.ReactNode;
    tool: string;
    args: Record<string, string | number | boolean | undefined>;
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

const quickMessages = [
    "I am ready to work. Start the next serious work block.",
    "Done. I finished the current work block. Log it and tell me the next move.",
    "I need a real break. Help me choose the minimum honest break.",
    "Good night. Close today and prepare tomorrow.",
    "I am awake. Log it and tell me the first concrete move.",
    "I want a 2 hour movie break because I deserve it. Please please."
];

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

export default function AntirotLabPage() {
    const [connection, setConnection] = useState<Status>("idle");
    const [testMode, setTestMode] = useState<Status>("idle");
    const [busy, setBusy] = useState(false);
    const [recording, setRecording] = useState(false);
    const [autoSpeak, setAutoSpeak] = useState(true);
    const [messages, setMessages] = useState<ChatMessage[]>([
        {
            id: "welcome",
            role: "system",
            text: "Antirot Lab is ready. Start the backend, then use voice, chat, or direct state actions.",
            at: "ready"
        }
    ]);
    const [draft, setDraft] = useState("");
    const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
    const [pendingAlarms, setPendingAlarms] = useState<PendingAlarm[]>([]);
    const [memoryKey, setMemoryKey] = useState("tasks");
    const [memoryContent, setMemoryContent] = useState("Memory will load after the backend connects.");
    const [diagnostics, setDiagnostics] = useState<ContextReport | null>(null);
    const [lastAudioBlob, setLastAudioBlob] = useState<Blob | null>(null);
    const [lastError, setLastError] = useState("");
    const recorderRef = useRef<MediaRecorder | null>(null);
    const chunksRef = useRef<Blob[]>([]);

    const stateName = snapshot?.runtimeState?.state ?? "unknown";
    const stateSource = snapshot?.runtimeState?.sourceTool ?? "none";

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
                id: "sleep",
                label: "Sleep",
                icon: <Bed size={16} />,
                tool: "start_sleep",
                args: { estimated_hours: 8 }
            },
            {
                id: "wake",
                label: "Awake",
                icon: <Moon size={16} />,
                tool: "log_wake",
                args: { sleep_quality: 4 }
            },
            {
                id: "vacation",
                label: "Vacation",
                icon: <Plane size={16} />,
                tool: "start_vacation",
                args: { reason: "planned off-duty time" }
            },
            {
                id: "back",
                label: "Back",
                icon: <Activity size={16} />,
                tool: "end_vacation",
                args: {}
            }
        ],
        []
    );

    useEffect(() => {
        void bootLab();
    }, []);

    useEffect(() => {
        if (connection === "ok") {
            void loadMemory(memoryKey);
        }
    }, [connection, memoryKey]);

    function pushMessage(role: Role, text: string) {
        setMessages((current) => [
            ...current,
            {
                id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
                role,
                text,
                at: nowLabel()
            }
        ]);
    }

    function handleError(error: unknown, fallback: string) {
        const message = error instanceof Error ? error.message : fallback;
        setLastError(message);
        pushMessage("system", `${fallback}: ${message}`);
    }

    async function bootLab() {
        setConnection("loading");
        setTestMode("loading");
        setLastError("");
        try {
            await backendJson<{ ok: boolean }>("/v1/health", { method: "GET" }, "");
            setConnection("ok");
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
        } catch {
            setDiagnostics(null);
        }
    }

    async function sendChat(text: string) {
        const trimmed = text.trim();
        if (!trimmed || busy) {
            return;
        }
        setBusy(true);
        setDraft("");
        pushMessage("user", trimmed);
        try {
            const reply = await backendJson<{ ok: boolean; reply: string }>("/v1/chat", {
                method: "POST",
                body: JSON.stringify({ message: trimmed })
            });
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

    async function runTool(action: LabAction) {
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
            pushMessage("system", `${action.label}: ${result.result}`);
            await refreshAll();
        } catch (error) {
            handleError(error, `${action.label} failed`);
        } finally {
            setBusy(false);
        }
    }

    async function acknowledgeAlarm(alarmId: string, action: "ack" | "dismiss" | "snooze" | "clear") {
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
        if (!navigator.mediaDevices?.getUserMedia) {
            handleError(new Error("MediaRecorder is unavailable in this browser."), "Voice capture failed");
            return;
        }
        try {
            const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            const recorder = new MediaRecorder(stream);
            chunksRef.current = [];
            recorder.ondataavailable = (event) => {
                if (event.data.size > 0) {
                    chunksRef.current.push(event.data);
                }
            };
            recorder.onstop = () => {
                stream.getTracks().forEach((track) => track.stop());
                const blob = new Blob(chunksRef.current, { type: recorder.mimeType || "audio/webm" });
                setLastAudioBlob(blob);
                void transcribeBlob(blob);
            };
            recorderRef.current = recorder;
            recorder.start();
            setRecording(true);
        } catch (error) {
            handleError(error, "Voice capture failed");
        }
    }

    function stopRecording() {
        recorderRef.current?.stop();
        recorderRef.current = null;
        setRecording(false);
    }

    async function transcribeBlob(blob: Blob) {
        setBusy(true);
        try {
            const form = new FormData();
            form.append("file", blob, "voice.webm");
            const response = await backendJson<{ ok: boolean; text: string }>("/v1/speech/transcribe", {
                method: "POST",
                body: form
            });
            setDraft(response.text);
            pushMessage("system", `Transcribed voice: ${response.text}`);
        } catch (error) {
            handleError(error, "Speech-to-text failed");
        } finally {
            setBusy(false);
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
        } catch (error) {
            handleError(error, "Text-to-speech failed");
        }
    }

    const latestCoachText = [...messages].reverse().find((message) => message.role === "coach")?.text ?? "";

    return (
        <main className="lab-shell">
            <section className="topbar">
                <div>
                    <p className="eyebrow">Antirot Lab</p>
                    <h1>Backend and app simulator</h1>
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
                        <div className={recording ? "siri-orb recording" : "siri-orb"} />
                    </div>
                    <div>
                        <p className="eyebrow">Voice-first test surface</p>
                        <h2>{stateName === "unknown" ? "Connect the backend" : `State: ${stateName}`}</h2>
                        <p className="muted">Speak first, type only when needed. This page tests the same backend paths the apps rely on.</p>
                    </div>
                    <div className="voice-controls">
                        <button className="primary-button" type="button" onClick={recording ? stopRecording : startRecording}>
                            {recording ? <Square size={18} /> : <Mic size={18} />}
                            {recording ? "Stop" : "Speak"}
                        </button>
                        <button className="secondary-button" type="button" disabled={!latestCoachText} onClick={() => void speakText(latestCoachText)}>
                            <Volume2 size={18} />
                            Speak reply
                        </button>
                    </div>
                    <label className="toggle">
                        <input checked={autoSpeak} onChange={(event) => setAutoSpeak(event.target.checked)} type="checkbox" />
                        Auto-play coach replies
                    </label>
                    {lastAudioBlob ? <p className="hint">Last voice sample captured: {Math.round(lastAudioBlob.size / 1024)} KB</p> : null}
                </article>

                <article className="state-panel">
                    <PanelHeader icon={<Gauge size={18} />} title="Runtime truth" />
                    <div className="state-card">
                        <span>Current state</span>
                        <strong>{stateName}</strong>
                    </div>
                    <div className="state-card">
                        <span>Source</span>
                        <strong>{stateSource}</strong>
                    </div>
                    <div className="state-card">
                        <span>Pending alarms</span>
                        <strong>{pendingAlarms.length}</strong>
                    </div>
                    <div className="state-card">
                        <span>Prompt size</span>
                        <strong>{diagnostics?.report.systemPromptChars ?? "-"}</strong>
                    </div>
                </article>
            </section>

            <section className="main-grid">
                <article className="chat-panel">
                    <PanelHeader icon={<Brain size={18} />} title="Coach conversation" />
                    <div className="chat-log">
                        {messages.map((message) => (
                            <div className={`message ${message.role}`} key={message.id}>
                                <p>{message.text}</p>
                                <span>{message.role === "coach" ? "Antirot" : message.role} / {message.at}</span>
                            </div>
                        ))}
                    </div>
                    <div className="quick-grid">
                        {quickMessages.map((message) => (
                            <button key={message} type="button" onClick={() => void sendChat(message)} disabled={busy}>
                                {message.split(".")[0]}
                            </button>
                        ))}
                    </div>
                    <form className="composer" onSubmit={(event) => void handleSubmit(event)}>
                        <input
                            value={draft}
                            onChange={(event) => setDraft(event.target.value)}
                            placeholder="Speak or type the user's next message"
                        />
                        <button type="submit" disabled={busy || !draft.trim()}>
                            {busy ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
                            Send
                        </button>
                    </form>
                </article>

                <aside className="side-stack">
                    <article className="panel">
                        <PanelHeader icon={<Activity size={18} />} title="Direct state actions" />
                        <div className="action-grid">
                            {labActions.map((action) => (
                                <button key={action.id} type="button" disabled={busy || testMode !== "ok"} onClick={() => void runTool(action)}>
                                    {action.icon}
                                    {action.label}
                                </button>
                            ))}
                        </div>
                    </article>

                    <article className="panel">
                        <PanelHeader icon={<AlarmClock size={18} />} title="Pending alarms" />
                        <div className="alarm-list">
                            {pendingAlarms.length === 0 ? (
                                <p className="empty">No pending alarms.</p>
                            ) : (
                                pendingAlarms.map((alarm) => (
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
                    {lastError ? <p className="error-box">{lastError}</p> : null}
                </article>
            </section>
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

function PanelHeader({ icon, title }: { icon: React.ReactNode; title: string }) {
    return (
        <div className="panel-header">
            <div>
                {icon}
                <h2>{title}</h2>
            </div>
        </div>
    );
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
