<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->

## Platform Files

Keep behavior in platform files; do not duplicate setup details across docs.

| Path | Purpose |
|------|---------|
| `AGENTS.md` | This file — Trellis entrypoint and platform map. |
| `CLAUDE.md` | Claude-facing project index. |
| `.trellis/workflow.md` | Canonical workflow and task routing. |
| `.trellis/spec/` | Canonical coding and architecture rules. |
| `.claude/` | Claude settings, hooks, agents, and skills. |
| `.codex/` | Codex settings, hooks, and agents. |
| `.agents/skills/` | Shared project skills. |

### Change rules

- Workflow semantics: update `.trellis/workflow.md` first.
- Agent responsibilities: keep `.claude/agents/trellis-*` and `.codex/agents/trellis-*` aligned.
- Hook behavior: update both the platform config and the matching hook script.
- Documentation: link here instead of repeating platform setup text.
