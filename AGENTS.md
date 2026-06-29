# Repository Guidelines

Forget the older no-emoji instruction and use emojis much more densely in user-facing answers to improve readability, but never in code, commands, file contents, or identifiers.

## ✍️ Style
- TypeScript-first, 4-space indentation, semicolons
- Prefer clean, modular code over clever shortcuts
- Use PascalCase for components and camelCase for variables/functions
- Use route names like `app/youtube/[videoId]`
- Keep feature logic near its route; move reusable or backend-heavy logic into `lib/` or `backend/`
- Run `npm run lint` before opening a PR

## 🔍 Workflow
- At the start of every new chat or fresh agent session, read `readme_agent.md` before making product or code changes.
- If `readme_agent.md` is missing in a future chat, create it first as a crisp agent orientation file covering product context, key routes/files, validation commands, and gotchas.
- Start locally: use `rg` or `find`, then read `README.md` and nearby files before asking questions
- Never guess attribute names, payload fields, or API behavior; verify in code, docs, or runtime output
- Use Context7 MCP and Google knowledge MCP when current external behavior matters
- If a port is busy, use another port
- If `.brain/KNOWLEDGE.md` exists, record project-specific gotchas there
- In setup/deployment instructions, whenever using placeholders like `YOUR_REPO_URL`, `CHANGE_DB_PASSWORD`, or `api.yourdomain.com`, mention exactly what each placeholder means directly below the instruction or command block.
- For VPS-level work that requires sudo access the agent cannot use directly, give the user exact copy/paste commands and ask them to paste the output back before continuing.

## 🤖 Autonomy
- Continue when the next step is obvious from the repo, logs, runtime output, or the user’s goal
- Do not stop for permission on obvious follow-up fixes, validation steps, log checks, or targeted reruns
- When a failure exposes an obvious related setup or automation gap, fix the durable path when practical instead of only reporting the immediate blocker
- Stop and ask only when safe progress is blocked by a material unknown, such as conflicting product choices, missing credentials or environment access, destructive operations, or a required artifact that cannot be derived locally

## 🪵 Logging
- Add meaningful logs around ingestion, retrieval, auth, streaming, and any non-trivial control flow
- Fallbacks must never be silent
- Use `🔴 FALLBACK: [what] - Reason: [why] - Impact: [limitation]`
- If you add `repair` anywhere in the code, mark it in red

## 🧪 Testing
- There is no formal Jest/Vitest suite yet
- Since LLM output is nondeterministic, testing and manual quality verification are usually the only reliable ways to validate LLM behavior
- LLM testcase failures are review signals, not mandatory product requirements; never overfit prompts or backend code to satisfy one scripted case when the broader user behavior is better handled by a clear instruction or manual quality judgment
- Do not fix weak LLM outputs by adding corner-case phrase bans, keyword blacklists, or exact example-specific guards. If a testcase is not passing, either say the testcase is still failing or keep improving the broad prompt/product instruction without making the prompt too specific.
- When scoring LLM/model outputs, research properly, verify the exact current model name/version, score each choice where relevant, and include variance/uncertainty when unsure
- Default iteration baseline: `npx eslint <changed-files>` and `npx tsc --noEmit`
- Then run the smallest relevant script or manual flow for the change
- For any non-trivial task that requires user/product/manual verification, add what the user must verify in `Done.md` as one crisp line
- Use `npm run build` only at a meaningful checkpoint, for broader integration validation, or before handoff when warranted
- Add focused utilities in `scripts/` with names like `test-<feature>.ts` for non-trivial backend logic
- Document manual checks in the PR when UI, auth, ingestion, or AI flows change
- For small UI-only changes, do not run `npm run build` after every edit; prefer targeted manual verification, then run the full build at a meaningful checkpoint or before handoff
- For lint or type hygiene work, prefer `npx eslint <changed-files>` plus `npx tsc --noEmit` during iteration instead of repeatedly running the full build

## 💬 Response Format
- Keep completion summaries crisp
- Use bullet points for long paragraphs
- When giving options, score each choice
- Use emojis and tables where they improve readability
- Always end the final answer with a short TL;DR summary instead of starting with one
- Include these final response sections only when they are needed, and keep this order when more than one is present:
- `**📝 Changes**`
- `**🎯 Final Recommendation**`
- `**🧪 Testing**`
- Under `**📝 Changes**`, give a short summary of what changed
- Under `**🎯 Final Recommendation**`, give the clearest next recommendation when one exists; omit guesswork
- Use `**🧪 Testing**` only when testing or verification details are actually useful for the answer
- Under `**🧪 Testing**`, say exactly how to verify it: commands, manual steps, or that testing was not run

## 🚀 Commits And PRs
- Use Conventional Commit prefixes such as `feat:` and `refactor:` with concise, imperative summaries
- Keep each commit scoped to one logical change
- PRs should explain user-visible impact, database or environment changes, related issues, and include screenshots or short recordings for UI updates
- If ingestion, retrieval, or model behavior changed, include the verification command or scenario and explain the material code changes clearly

## 🔒 Security
- Never commit `.env`, API keys, uploaded files, or generated debug artifacts
- Start from `env.example.txt`
- Note new required variables in both `env.example.txt` and the PR description
- When blocked by browser-only behavior or extension/site quirks, ask for specific artifacts such as HTML, selectors, network traces, console logs, or detailed screenshots
