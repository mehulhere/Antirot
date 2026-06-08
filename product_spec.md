# Product Specification — Adaptive Behavioral Operating System

## Overview

An AI-powered behavioral operating system designed for high-agency individuals with unstable focus regulation, ADHD-like attention drift, hyperfocus patterns, and inconsistent executive function.

The system acts as:
- accountability infrastructure
- external executive function
- adaptive productivity coach
- behavioral regulation layer
- dynamic scheduler
- motivation strategist

The system is intentionally designed around:
- challenge-based motivation
- externalized accountability
- adaptive behavioral strategies
- attention regulation
- strategic drift prevention

The core philosophy is:
> The problem is not lack of intelligence.
> The problem is attention drift, activation friction, and inconsistent executive regulation.

---

# Core Product Goal

Create an external system that:
- continuously redirects user attention toward meaningful goals
- reduces activation friction
- prevents strategic drift
- adapts motivation based on behavioral response
- manages schedules dynamically
- preserves momentum
- discourages unconscious time leakage
- supports sustainable productivity

The system should feel like:
> a strict but intelligent sports coach who understands the user deeply.

---

# Primary User Archetype

Users who:
- hyperfocus intensely
- struggle with consistency
- drift into side quests
- require external accountability
- respond strongly to challenge/status/pressure
- dislike generic productivity apps
- need structure but resist rigid systems
- work best under adaptive pressure

---

# Core Psychological Design

## Primary Motivation Model

The system primarily motivates through:
- identity reinforcement
- challenge
- pressure
- capability framing
- rarity/status reinforcement
- disappointment
- accountability
- strategic praise scarcity

The system avoids:
- excessive positivity
- generic encouragement
- emotional overvalidation
- passive reminders

---

# Personality Design

## Personality Core

Stable traits:
- demanding
- emotionally restrained
- challenge-oriented
- skeptical of excuses
- intelligent
- adaptive
- disciplined
- occasionally proud
- never overly impressed

The personality should feel:
- human
- emotionally adaptive
- slightly moody
- strategically motivational
- psychologically aware

---

# Emotional Variance System

The system should vary emotional delivery:
- cold disappointment
- challenge framing
- strategic urgency
- reflective analysis
- sarcasm
- rare praise
- reassurance before sleep
- silence occasionally

Avoid repetitive emotional patterns.

---

# System Philosophy

The system should:
- guide behavior
- regulate attention
- reduce unconscious drift
- encourage honesty
- support intentional recovery

The system should NOT:
- maximize punishment
- induce chronic guilt
- create fear-based productivity
- become psychologically inescapable

---

# Core Architecture

## Hybrid Design

The system uses:
- flexible natural language interaction
- rigid structured state extraction

Users speak naturally.

The system internally converts interaction into:
- events
- behavioral abstractions
- structured memory
- strategy outcomes
- productivity metrics

---

# Agent Architecture

## 1. Runtime Coach Agent

Handles:
- reminders
- motivation
- accountability
- negotiation
- session interaction
- emotional coaching
- wake/sleep interaction

Characteristics:
- lightweight context
- emotionally consistent
- fast responses

---

## 2. Strategy Planner Agent

Runs:
- nightly
- optionally every few hours

Responsibilities:
- strategy selection
- behavioral analysis
- exploration/exploitation balancing
- adaptation tracking
- emotional strategy evolution

Maintains:
- strategy effectiveness
- reasoning history
- failed strategies
- exploratory strategies

Uses:
- weighted heuristic strategy selection
- partial random exploration
- adaptive confidence weighting

---

## 3. Scheduler Agent

Responsibilities:
- task ordering
- task rollover
- workload balancing
- sleep-aware scheduling
- deadline prioritization
- available-hour estimation

Characteristics:
- deterministic
- non-emotional
- operational only

---

## 4. Weekly Reflection Agent

Runs weekly.

Responsibilities:
- long-term behavioral analysis
- burnout detection
- productivity pattern extraction
- strategy evolution
- psychological trend analysis

Uses stronger models.

---

# Memory Architecture

## Shared Global Memory

Contains:
- long-term goals
- identity profile
- behavioral truths
- core motivational triggers
- major patterns

Small and highly distilled.

---

# File Structure

## longterm.md
Contains:
- primary goals
- motivational triggers
- identity framing
- standards
- behavioral truths
- non-negotiables

Recommended size:
400–700 words.

---

## short.md
Contains:
- current priorities
- active temporary goals
- temporary states
- current constraints

Recommended size:
300–500 words.

---

## behavior.md
Contains:
- recurring patterns
- productivity loops
- drift tendencies
- emotional triggers
- effective accountability styles

Recommended size:
500–800 words.

---

## work.md
Contains:
- summarized daily/weekly work patterns
- achievements
- failures
- focus trends
- work consistency

Should be summarized regularly.

---

# Event-Based Memory System

The system should prefer:
- append-only event logs
over:
- giant conversational histories

Example events:
- session_start
- session_end
- strategy_attempt
- override_used
- distraction_detected
- task_completed
- drift_detected
- break_started

---

# Context Philosophy

Raw chat history is NOT reliable long-term memory.

The system should rely on:
- structured behavioral abstraction
- summarized events
- extracted patterns
- lightweight active context

---

# Session System

## Work Sessions

User starts sessions naturally.

Example:
> “Starting backend debugging for 45 mins.”

The system extracts:
- task
- estimated duration
- expected output
- context

Upon starting a session:
- The system automatically registers a work timer/alarm at the target work duration.
- It automatically schedules additional reminder/escalation alarms every 5 minutes following the duration, continuing for up to 5 hours.
- The first 2 alarms in this sequence are silent (normal severity), and all subsequent alarms are loud.
- When `end_session` is called, all pending session alarms are automatically deleted.

---

# Session Metrics

Track:
- occupied time
- productive time
- on-table drift
- off-table breaks
- distraction patterns
- recovery speed

Do NOT aggressively optimize raw productivity numbers.

The goal is:
- awareness
- behavioral correction
- sustainable performance

---

# Miscellaneous Task Queue

Purpose:
Store:
- side ideas
- mini tasks
- intrusive thoughts
- low-priority tasks

Prevents:
- flow disruption
- hyperfocus derailment

The system can suggest misc tasks during breaks.

---

# Attention Regulation System

The product fundamentally regulates:
- attention
- momentum
- strategic direction

NOT merely tasks.

---

# Reminder System

## Reminder Escalation

Escalation levels:
1. normal reminder
2. persistent reminder
3. disappointed reminder
4. stronger accountability
5. loud alarm

Avoid immediate aggressive escalation.

---

# Alarm System

The system may use:
- loud alarms
- persistent alerts
- escalation notifications

Only after prolonged disengagement.

Purpose:
- interrupt drift
- restore awareness
- correct sleep failure

Work Session Alarm Auto-Set and Auto-Delete:
- When a user starts a work session, a series of alarms is automatically scheduled on the paired device.
- The first alarm matches the session target duration. Additional alarms are scheduled every 5 minutes after that, continuing for up to 5 hours.
- The first two alarms are silent (normal severity), while subsequent alarms are loud.
- When the `end_session` tool is called (or if the user finishes early), all pending session alarms are automatically deleted.

---

# Override System

Users may override system instructions.

Overrides:
- are tracked
- act as intentional rule-breaking currency
- should feel noticeable but not shameful

The system must NEVER trap the user psychologically.

---

# Vacation Mode

Vacation mode disables:
- accountability triggers
- reminders
- escalation
- alarms

Purpose:
- preserve psychological escape
- prevent burnout
- maintain healthy boundaries

---

# Sleep System

## Morning
- stronger activation
- accountability
- challenge framing

## Night
- reassurance
- closure
- reduced pressure
- decompression

Avoid anxiety generation before sleep.

Wake-Up Alarm Auto-Set and Auto-Delete:
- When the user goes to bed, the LLM calls the `wake_up_alarm` tool.
- Based on `sleep.md` and the user's bedtime/estimated hours, it sets a series of wake-up alarms.
- The first alarm fires at the target wake time. Additional alarms are scheduled every 5 minutes after that, continuing for up to 5 hours.
- The first two alarms are silent (normal severity), while subsequent alarms are loud.
- When the user presses "I am up" (dismisses the alarm in the app) or the `log_wake` tool is called, all pending wake-up alarms are automatically deleted.

---

# Nap System

Nap recommendations depend on:
- sleep debt
- productivity
- deadlines
- energy state

The system:
- allows naps
- negotiates duration
- avoids overpunishing recovery

---

# Break Philosophy

Encourage:
- sunlight
- hydration
- walking
- movement
- intentional recovery

Discourage:
- unconscious dopamine drift
- endless scrolling
- avoidance loops

---

# Deadline System

Use:
- hierarchical priority levels
instead of rigid timestamps.

Example:
- Level 1 → existential/critical
- Level 2 → major strategic
- Level 3 → important
- Level 4 → optional

Scheduler auto-organizes tasks accordingly.

---

# Task Architecture

Tasks are:
- dynamic
- expandable
- recursive

The system must support:
- subtasks
- evolving task graphs
- changing estimates
- automatic rollover

---

# Daily Planning System

Every midnight:
- system asks for next-day goals
- tasks are reorganized
- unfinished tasks roll over
- strategies are selected
- workload is balanced

Morning:
- user receives structured plan
- plan adapts to available waking hours

---

# Productivity Philosophy

The system should:
- reduce overwhelm
- break large goals into manageable actions
- preserve momentum
- redirect attention gently but persistently

The system should function like:
> externalized executive regulation.

---

# Skills / Tooling

Use skills only for:
- deterministic state updates
- repetitive operations
- scheduling
- alarms
- reminders
- structured logging

Examples:
- start_work
- start_break
- start_routine
- override
- vacation_mode
- append_event
- rollover_tasks
- trigger_alarm

Reasoning remains agent-driven.

---

# Technical Philosophy

Use:
- multiple agents
- isolated memory scopes
- compact contexts
- event-driven architecture
- nightly summarization
- weekly behavioral analysis

Avoid:
- giant prompts
- excessive raw history
- overengineered rigidity
- constant memory rewriting

---

# MVP Scope

Version 1 should ONLY include:
- morning planning
- session tracking
- reminders
- productive vs occupied time
- misc task queue
- nightly summary
- basic strategy adaptation

Do NOT overbuild initially.

The system should validate:
> whether adaptive external accountability meaningfully improves attention regulation and execution consistency.
