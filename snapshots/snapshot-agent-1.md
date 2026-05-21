# Snapshot: snapshot-agent-1

**Saved:** 2026-05-21T09:14:10Z
**Project:** C:\Users\mkibb\Documents\deep-meteor\architext-core
**Status:** ACTIVE

## Objective

Diagnose why the `snapshot-agent` plugin is not appearing in the Antigravity CLI `/agents` menu, and determine whether to fix it or accept skill-based invocation as sufficient.

## Completed Milestones

- [x] Previous session (bca5a565) created `.agents/plugins/snapshot-agent/plugin.json` and `.agents/plugins/snapshot-agent/skills/snapshot/SKILL.md`
- [x] Confirmed the snapshot **skill** IS loading correctly — visible in agent context as `snapshot` skill from `snapshot-agent` plugin
- [x] Confirmed `phase3-layout-ux.md` snapshot exists and is intact at `.agents/skills/snapshots/phase3-layout-ux.md`
- [x] Diagnosed root cause: **skills ≠ subagents** — skills are invoked contextually; subagents appear in `/agents` menu
- [x] Identified fix: need an `agents/` subdirectory inside the plugin with a subagent definition `.md` file

## Pending Next Step

User must decide between two options before we continue:

1. **Do nothing** — snapshot skill already works via natural language ("snapshot this as X"), no menu entry needed
2. **Create subagent definition** — add `.agents/plugins/snapshot-agent/agents/snapshot-agent.md` to make it appear in the `/agents` menu

After that decision: consider whether to load `phase3-layout-ux.md` to resume the Architext Phase 3 UI work.

## Active Constraints

- The snapshot skill saves to `.agents/skills/snapshots/<name>.md` — do not change this path
- The plugin structure at `.agents/plugins/snapshot-agent/` is correct and should not be moved
- Subagent definitions for the `/agents` menu require a file inside an `agents/` subdirectory of the plugin folder
- The `.agents/snapshot-agent.md` at the project root is a legacy artifact — it is NOT what the CLI scans

## File References

| File                                                      | Purpose                         | Notes                                               |
| --------------------------------------------------------- | ------------------------------- | --------------------------------------------------- |
| `.agents/plugins/snapshot-agent/plugin.json`              | Plugin registration marker      | Must exist; currently `{"name": "snapshot-agent"}`  |
| `.agents/plugins/snapshot-agent/skills/snapshot/SKILL.md` | Skill definition                | Loaded correctly; drives contextual invocation      |
| `.agents/skills/snapshots/phase3-layout-ux.md`            | Saved Architext Phase 3 context | Ready to load and resume Phase 3 UI work            |
| `.agents/snapshot-agent.md`                               | Legacy file                     | Not scanned by CLI — can be deleted or kept as docs |

## Command History

```bash
# No commands run this session — all work was read/diagnostic
```

## Open Questions

- Does the user want the snapshot-agent to appear in the /agents menu (requires creating agents/ subdir + definition file)?
- Should we load phase3-layout-ux.md and resume Architext Phase 3 UI work after resolving this?
