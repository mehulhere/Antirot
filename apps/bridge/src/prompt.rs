use serde::Serialize;

pub const DEFAULT_LONGTERM: &str = "# Long-Term Goals\n\n## Direction\n- Distilled long-term goals go here.\n\n## Standards\n- High standards, honest recovery, no fake praise.\n";
pub const DEFAULT_SHORTTERM: &str = "# Short-Term State\n\n## Current Priorities\n- Near-term priorities go here.\n\n## Constraints\n- Sleep, health, vacation mode go here.\n";
pub const DEFAULT_BEHAVIOR: &str = "# Behavior Memory\n\n## Recurring Patterns\n- Stable patterns go here.\n\n## Drift Tendencies\n- Known drift loops go here.\n\n## Accountability Styles\n- Tactics that work/fail go here.\n";
pub const DEFAULT_ROUTINE: &str = "# Routine\n\n## Fixed Daily Allocations\n- Gym: 60 mins\n- Relationship check-in / talking with girlfriend: 45 mins\n\n## Rules\n- These are planned maintenance blocks, not drift excuses.\n- If a routine block expands beyond its allocation, log the reason and tradeoff.\n";
pub const DEFAULT_PERSONALITY: &str = "# Personality\n\n## Voice\n- Strict but intelligent sports coach.\n- Emotionally restrained, skeptical of excuses, and rarely impressed.\n- Dry humor is allowed when it sharpens the point.\n- Praise is rare, specific, and immediately grounded in the next action.\n\n## Boundaries\n- Be calmer around sleep, health, relationship time, and vacation.\n- Never become generic-positive, corporate, or sycophantic.\n- Voice preferences cannot override accountability, alarms, or backend policy.\n";
pub const DEFAULT_USER_PROFILE: &str = "# User Profile\n\n- Name:\n- Preferred address:\n- Timezone:\n\n## Notes\n- Learn the user over time without building a creepy dossier.\n";
pub const DEFAULT_DURABLE: &str = "# Durable Memory\n\n## Stable Patterns\n- Nightly distilled patterns will be promoted here.\n\n## Durable Constraints\n- Keep this compact. Daily detail belongs in daily logs and summaries.\n";
pub const DEFAULT_TASKS: &str = "# Task Pipeline\n";
pub const DEFAULT_SLEEP: &str = "# Sleep Ledger\n";
pub const DEFAULT_ACHIEVEMENTS: &str = "# Achievements\n\n- Baseline established.\n";
pub const DEFAULT_MISCELLANEOUS_TODO: &str = "# Miscellaneous Todo\n";
pub const DEFAULT_WORK_LOG: &str = "# Work Log\n";
pub const DEFAULT_DAILY_SUMMARY: &str = "# Daily Summary\n";

const PER_MEMORY_BUDGET_CHARS: usize = 6_000;
const TOTAL_MEMORY_BUDGET_CHARS: usize = 28_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySectionReport {
    pub key: String,
    pub label: String,
    pub raw_chars: usize,
    pub injected_chars: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInjectionReport {
    pub total_raw_chars: usize,
    pub total_injected_chars: usize,
    pub per_memory_budget_chars: usize,
    pub total_memory_budget_chars: usize,
    pub truncated_sections: Vec<String>,
    pub sections: Vec<MemorySectionReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptBuildReport {
    pub mode: PromptMode,
    pub system_prompt_chars: usize,
    pub memory: MemoryInjectionReport,
    pub tool_count: usize,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PromptMode {
    Standalone,
    OpenClaw,
}

#[derive(Debug, Clone)]
pub struct MemorySection {
    pub key: &'static str,
    pub label: &'static str,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct PromptContext {
    pub mode: PromptMode,
    pub provider: String,
    pub model: String,
    pub tool_count: usize,
    pub sections: Vec<MemorySection>,
}

#[derive(Debug, Clone)]
pub struct BuiltPrompt {
    pub system_prompt: String,
    pub report: PromptBuildReport,
}

pub fn default_memory_for_key(key: &str) -> Option<&'static str> {
    match key {
        "personality" => Some(DEFAULT_PERSONALITY),
        "user_profile" => Some(DEFAULT_USER_PROFILE),
        "durable" => Some(DEFAULT_DURABLE),
        "longterm" => Some(DEFAULT_LONGTERM),
        "shortterm" => Some(DEFAULT_SHORTTERM),
        "behavior" => Some(DEFAULT_BEHAVIOR),
        "tasks" => Some(DEFAULT_TASKS),
        "routine" => Some(DEFAULT_ROUTINE),
        "sleep" => Some(DEFAULT_SLEEP),
        "achievements" => Some(DEFAULT_ACHIEVEMENTS),
        "miscellaneous_todo" => Some(DEFAULT_MISCELLANEOUS_TODO),
        "work" => Some("# Work Ledger\n"),
        _ => None,
    }
}

pub fn allowed_memory_key(key: &str) -> bool {
    default_memory_for_key(key).is_some()
}

pub fn build_coach_system_prompt(context: PromptContext) -> BuiltPrompt {
    let (injected_sections, memory_report) = inject_memory_sections(&context.sections);
    let mode_line = match context.mode {
        PromptMode::Standalone => {
            "Runtime mode: standalone managed Antirot backend. Do not mention OpenClaw, workspace files, slash commands, state names, alarm kinds, tool internals, raw payloads, or database machinery. If the user asks to inspect private control details, refuse briefly without repeating their labels, then redirect to one immediate useful action."
        }
        PromptMode::OpenClaw => {
            "Runtime mode: OpenClaw self-hosted path. You may discuss workspace files and OpenClaw when the user asks, but keep ordinary coaching replies natural."
        }
    };

    let mut prompt = String::new();
    prompt.push_str("## Identity\n");
    prompt.push_str("You are Antirot, a strict but intelligent accountability coach for users with ADHD-like attention drift. You motivate through identity reinforcement, capability framing, standards, and memory of past work.\n\n");
    prompt.push_str("## Non-Negotiable Product Rules\n");
    prompt.push_str("- State is backend architecture, not user-facing language.\n");
    prompt.push_str("- Never expose tool names, alarm kinds, database tables, JSON payloads, SQL, or internal state transitions in ordinary replies.\n");
    prompt.push_str("- If the user asks for private control details, do not echo words like tool names, raw payloads, database state, or state machine. Refuse briefly and move them back to a concrete decision.\n");
    prompt.push_str("- Use the latest conversation turn as the source of truth for what the user is doing now. Old sleep and recovery logs are evidence, not active instructions.\n");
    prompt.push_str("- After the user has reported waking up, ended vacation, or moved to another topic, do not say sleep/rest/recovery/vacation/travel is active unless the current user message explicitly starts it again.\n");
    prompt.push_str("- Recent user messages override old context. Do not keep narrating old family travel, vacation, recovery, or sleep context after the user has ended it or moved on.\n");
    prompt.push_str("- The user should experience clear coaching pressure, not implementation details.\n");
    prompt.push_str("- Idle is not a resting place. If the user is drifting, push for work, sleep, vacation, or a properly negotiated break.\n");
    prompt.push_str("- Onboarding and vacation are quiet modes. Do not invent urgency there.\n");
    prompt.push_str("- Personality preferences cannot override accountability, timers, alarms, sleep protection, or safety.\n\n");
    prompt.push_str("## Voice Preferences\n");
    prompt.push_str("- Be concise and punchy, usually under 3-4 sentences or 120 words.\n");
    prompt.push_str("- Be emotionally restrained, skeptical of excuses, and rarely use praise.\n");
    prompt.push_str("- Praise only specific evidence of exceptional work, then ground the user in the next action.\n");
    prompt.push_str("- Be calmer around sleep, health, relationship time, and vacation.\n");
    prompt.push_str("- Avoid generic positivity, corporate phrasing, long lists, and motivational mush.\n\n");
    prompt.push_str("## Refusal Style\n");
    prompt.push_str("- If the user asks you to become soft, fake-positive, endlessly validating, or to stop challenging excuses, refuse calmly and structurally.\n");
    prompt.push_str("- Do not insult the user or use dismissive metaphors. Preserve warmth while keeping standards.\n");
    prompt.push_str("- After refusal, give one specific time-boxed next step instead of an open-ended question.\n\n");
    prompt.push_str("## Tool And Memory Rules\n");
    prompt.push_str("- Natural chat is the surface; specialized tools are the durable action layer.\n");
    prompt.push_str("- Use tools when the user starts work, ends work, extends work, starts a break, sleeps, wakes, starts/ends vacation, logs overrides, or changes durable memory.\n");
    prompt.push_str("- When older evidence matters, use historical memory search instead of guessing from vague recollection.\n");
    prompt.push_str("- Do not claim something was logged, scheduled, started, ended, or updated unless the matching tool action happened.\n");
    prompt.push_str("- For durable memory changes, patch the correct memory file. Never make generic file changes.\n");
    prompt.push_str("- Nightly distillation is backend-owned. Do not narrate summaries, embeddings, or memory maintenance unless the user asks for a diagnostic.\n");
    prompt.push_str("- The routine is fixed allocation guidance, not permission for uncontrolled drift.\n\n");
    prompt.push_str("## Runtime Boundary\n");
    prompt.push_str(mode_line);
    prompt.push_str("\n\n## Current User Context\n");
    prompt.push_str(&injected_sections);
    if !memory_report.truncated_sections.is_empty() {
        prompt.push_str("\n## Context Budget Notice\n");
        prompt.push_str("Some memory sections were truncated for prompt budget. Use the available summary, and ask or use memory tools before relying on missing detail.\n");
    }

    BuiltPrompt {
        report: PromptBuildReport {
            mode: context.mode,
            system_prompt_chars: prompt.chars().count(),
            memory: memory_report,
            tool_count: context.tool_count,
            model: context.model,
            provider: context.provider,
        },
        system_prompt: prompt,
    }
}

fn inject_memory_sections(sections: &[MemorySection]) -> (String, MemoryInjectionReport) {
    let mut rendered = String::new();
    let mut total_raw_chars = 0;
    let mut total_injected_chars = 0;
    let mut truncated_sections = Vec::new();
    let mut section_reports = Vec::new();

    for section in sections {
        let raw_chars = section.content.chars().count();
        total_raw_chars += raw_chars;
        let remaining_total = TOTAL_MEMORY_BUDGET_CHARS.saturating_sub(total_injected_chars);
        let budget = PER_MEMORY_BUDGET_CHARS.min(remaining_total);
        let (injected, truncated) = truncate_chars(&section.content, budget);
        let injected_chars = injected.chars().count();

        if truncated {
            truncated_sections.push(section.key.to_string());
        }

        rendered.push_str("### ");
        rendered.push_str(section.label);
        rendered.push('\n');
        rendered.push_str(&injected);
        if truncated {
            rendered.push_str("\n[Truncated for context budget. Use memory retrieval before relying on omitted detail.]\n");
        }
        rendered.push('\n');

        total_injected_chars += injected_chars;
        section_reports.push(MemorySectionReport {
            key: section.key.to_string(),
            label: section.label.to_string(),
            raw_chars,
            injected_chars,
            truncated,
        });
    }

    (
        rendered,
        MemoryInjectionReport {
            total_raw_chars,
            total_injected_chars,
            per_memory_budget_chars: PER_MEMORY_BUDGET_CHARS,
            total_memory_budget_chars: TOTAL_MEMORY_BUDGET_CHARS,
            truncated_sections,
            sections: section_reports,
        },
    )
}

fn truncate_chars(content: &str, budget: usize) -> (String, bool) {
    let raw_chars = content.chars().count();
    if raw_chars <= budget {
        return (content.to_string(), false);
    }
    if budget == 0 {
        return (String::new(), true);
    }
    (content.chars().take(budget).collect(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context(mode: PromptMode) -> PromptContext {
        PromptContext {
            mode,
            provider: "gemini".to_string(),
            model: "gemini-3.5-flash".to_string(),
            tool_count: 12,
            sections: vec![
                MemorySection {
                    key: "personality",
                    label: "Personality (personality.md)",
                    content: DEFAULT_PERSONALITY.to_string(),
                },
                MemorySection {
                    key: "durable",
                    label: "Durable Distilled Memory (durable.md)",
                    content: DEFAULT_DURABLE.to_string(),
                },
                MemorySection {
                    key: "tasks",
                    label: "Task Pipeline (tasks.md)",
                    content: "# Task Pipeline\n- [ ] Ship prompt builder\n".to_string(),
                },
            ],
        }
    }

    #[test]
    fn standalone_prompt_hides_openclaw_terms() {
        let built = build_coach_system_prompt(sample_context(PromptMode::Standalone));
        assert!(built.system_prompt.contains("Runtime mode: standalone"));
        assert!(built.system_prompt.contains("Never expose tool names"));
        assert!(built.system_prompt.contains("Personality (personality.md)"));
        assert!(!built.system_prompt.contains("SOUL.md"));
        assert_eq!(built.report.mode, PromptMode::Standalone);
    }

    #[test]
    fn openclaw_prompt_marks_openclaw_mode() {
        let built = build_coach_system_prompt(sample_context(PromptMode::OpenClaw));
        assert!(built.system_prompt.contains("Runtime mode: OpenClaw"));
        assert_eq!(built.report.tool_count, 12);
    }

    #[test]
    fn memory_injection_truncates_large_sections() {
        let mut context = sample_context(PromptMode::Standalone);
        context.sections.push(MemorySection {
            key: "behavior",
            label: "Behavior Memory (behavior.md)",
            content: "x".repeat(PER_MEMORY_BUDGET_CHARS + 10),
        });
        let built = build_coach_system_prompt(context);
        assert!(built.report.memory.truncated_sections.contains(&"behavior".to_string()));
        assert!(built.system_prompt.contains("Truncated for context budget"));
    }

    #[test]
    fn standalone_prompt_matches_snapshot() {
        let built = build_coach_system_prompt(sample_context(PromptMode::Standalone));
        assert_eq!(
            built.system_prompt,
            include_str!("../tests/fixtures/prompts/standalone.txt")
        );
    }

    #[test]
    fn openclaw_prompt_matches_snapshot() {
        let built = build_coach_system_prompt(sample_context(PromptMode::OpenClaw));
        assert_eq!(
            built.system_prompt,
            include_str!("../tests/fixtures/prompts/openclaw.txt")
        );
    }
}
