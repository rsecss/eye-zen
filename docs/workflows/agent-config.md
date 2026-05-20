# Agent Configuration

This is the map. Keep behavior in platform files; do not copy setup details into multiple docs.

| Path | Purpose |
|------|---------|
| `AGENTS.md` | Short Trellis entrypoint for AGENTS-aware tools. |
| `CLAUDE.md` | Claude-facing project index. |
| `.trellis/workflow.md` | Canonical workflow and task routing. |
| `.trellis/spec/` | Canonical coding and architecture rules. |
| `.claude/` | Claude settings, hooks, agents, and skills. |
| `.codex/` | Codex settings, hooks, and agents. |
| `.agents/skills/` | Shared project skills. |

## Change Rules

- Workflow semantics: update `.trellis/workflow.md` first.
- Agent responsibilities: keep `.claude/agents/trellis-*` and `.codex/agents/trellis-*` aligned.
- Hook behavior: update both the platform config and the matching hook script.
- Documentation: link here instead of repeating platform setup text.
