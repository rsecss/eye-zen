# Agent Configuration

This is the map. Keep behavior in platform files; do not copy setup details into multiple docs.

| Path                   | Purpose                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------- |
| `AGENTS.md`            | Shared project instructions, project index, and platform map for Claude Code and other agents. |
| `.trellis/workflow.md` | Canonical workflow and task routing.                                                           |
| `.trellis/spec/`       | Canonical coding and architecture rules.                                                       |
| `.claude/`             | Claude settings, hooks, agents, and skills.                                                    |
| `.codex/`              | Codex settings, hooks, and agents.                                                             |
| `.agents/skills/`      | Shared project skills.                                                                         |

Claude Code v2.1.277 and later can read `AGENTS.md` directly. This repository uses that shared entrypoint without a root `CLAUDE.md`. See the [official instruction-loading rules](https://code.claude.com/docs/en/memory#agents-md).

## Change Rules

- Workflow semantics: update `.trellis/workflow.md` first.
- Agent responsibilities: keep `.claude/agents/trellis-*` and `.codex/agents/trellis-*` aligned.
- Hook behavior: update both the platform config and the matching hook script.
- Documentation: link here instead of repeating platform setup text.
