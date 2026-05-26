# Contributing

Thanks for helping build Antirot.

## Local Setup

```bash
npm install
npm run build
npx openclaw plugins install --link .
npx openclaw plugins enable antirot
npx openclaw plugins inspect antirot --runtime --json
```

## Validation

Before opening a pull request, run:

```bash
npm run lint
npm run typecheck
npm run build
```

For behavior changes, also run the focused scenario script when relevant:

```bash
node scripts/test-scenarios.ts
```

## Repository Hygiene

Do not commit personal Antirot runtime memory or local secrets:

- `.env`
- `.antirot/`
- `behavior.md`
- `longterm.md`
- `shortterm.md`
- `tasks.md`
- `work.md`
- `sleep.md`
- `miscellaneous_todo.md`

Use Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`, and `refactor:`.
