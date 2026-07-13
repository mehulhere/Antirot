# Prompt and LLM orchestration

```mermaid
flowchart TD
    A["POST /v1/chat<br/>apps/backend/src/routes.rs:1484"] --> B["Authenticate user<br/>apps/backend/src/auth.rs:94"]
    B --> C["Resolve subscription/provider<br/>apps/backend/src/llm.rs:149"]
    C --> D["Load oldest 20 history rows<br/>apps/backend/src/llm.rs:259"]
    D --> E["Load state, memories, recall, metrics<br/>apps/backend/src/llm.rs:769"]
    E --> F["Build system prompt<br/>apps/backend/src/prompt.rs:157"]
    F --> G["Provider/tool loop<br/>apps/backend/src/llm.rs:345"]
    G -->|"tool call"| H["Execute local tool<br/>apps/backend/src/llm.rs:1347"]
    H --> I["Persist side effects and tool result<br/>apps/backend/src/llm.rs:457"]
    I --> G
    G -->|"final text"| J["Override with last curated reply<br/>apps/backend/src/llm.rs:512"]
    J --> K["Return reply plus runtime state<br/>apps/backend/src/routes.rs:1490"]
```

Key boundaries: PostgreSQL, Vertex/Gemini/OpenAI-compatible APIs, Gemini/Voyage embeddings, and alarm persistence. The visible reply can diverge from the persisted assistant reply at `apps/backend/src/llm.rs:431-516`.
