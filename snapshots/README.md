# Screenshot Feedback Workflow

## How to Use

1. **Take a screenshot** (Win+Shift+S, Snipping Tool, PrtScn — whatever you like)
2. **Drop it into `snapshots/inbox/`** — any filename is fine (`Screenshot 2026-05-21 151022.png`, `asdf.png`, whatever)
3. **Tell me** — just say "check inbox" or "new screenshot" or even just "look"
4. I will:
   - View every image in `inbox/`
   - Analyze the UI state, identify issues or wins
   - Rename it descriptively (e.g. `05_telemetry-modal-overlap-bug.png`)
   - Move it to `ui-feedback/` (active issues) or `resolved/` (confirmed working)
   - Respond with observations and next steps

## Folder Structure

```
snapshots/
├── inbox/           ← DROP ZONE: raw screenshots go here
├── ui-feedback/     ← Active: screenshots with open UX issues
├── resolved/        ← Archive: confirmed working states
├── _template.md     ← Snapshot template
└── *.md             ← Session snapshot documents
```

## Naming Convention (applied automatically)
Files are renamed to: `{NN}_{kebab-description}.png`
- `NN` = sequential number across all folders
- Description derived from what's visible in the screenshot

## Current Archive

| # | File | Status | Description |
|---|------|--------|-------------|
| 01 | `resolved/01_broken-ui-stub-binary.png` | ✅ Resolved | GUI binary was using dead stub, not full UI |
| 02 | `resolved/02_full-ui-restored.png` | ✅ Resolved | Full ArchitextApp wired, panels visible |
| 03 | `resolved/03_preview-and-editor-working.png` | ✅ Resolved | Chapter 8 preview rendering, file browser populated |
| 04 | `resolved/04_telemetry-dashboard-20ch.png` | ✅ Resolved | 108,141 words / 20 chapters telemetry confirmed |
| 05 | `resolved/05_agent-compilation-success.png` | ✅ Resolved | Terminal showing clean compile and successful launch of dev profile. |
| 06 | `resolved/06_completion-sprint-log.png` | ✅ Resolved | Terminal tracking progress of the Vespers 20-chapter completion sprint. |
| 07 | `resolved/07_subagent-bleed-analysis.png` | ✅ Resolved | Terminal showing deep analysis of subagent context bleed and layout formatting. |
| 08 | `resolved/08_assembled-telemetry-dashboard.png` | ✅ Resolved | The egui Rust GUI running side-by-side with our compiled manuscript telemetry. |
| 09 | `resolved/09_agent-monologue-and-inbox-focus.png` | ✅ Resolved | CLI terminal showing agent monologue, tool history, and targeted inbox focus. |
| 10 | `resolved/10_cli-table-and-ui-proposals.png` | ✅ Resolved | CLI terminal displaying the formatted screenshot archive table and next-step proposals. |
| 11 | `resolved/11_cli-metacognition-and-filing.png` | ✅ Resolved | CLI terminal showing the highlight on the Metacognition block and files 09 & 10 processed. |
| 12 | `resolved/12_cli-apocrypha-bouncer-run.png` | ✅ Resolved | CLI terminal showing the execution of the Apocrypha Bouncer snapshot flow. |
| 13 | `resolved/13_cli-scratchpad-framework-render.png` | ✅ Resolved | CLI terminal displaying the structured scratchpad framework formatting blocks. |
| 14 | `resolved/14_cli-final-sprint-summary.png` | ✅ Resolved | CLI terminal showing the completed sprint achievements and bounced technical stems. |
| 15 | `resolved/15_elevenreader-md-encoding-errors.png` | ✅ Resolved | ElevenReader rendering showing encoding bugs for raw emojis/em-dashes in Markdown format. |
| 16 | `resolved/16_elevenreader-table-pipes-spoken-bug.png` | ✅ Resolved | ElevenReader rendering showing spoken pipe characters and table dashes, destroying TTS quality. |
| 17 | `resolved/17_cli-overhaul-convert-pdf.png` | ✅ Resolved | CLI terminal detailing the overhauling of `convert_to_pdf.py` for TTS sanitization. |
| 18 | `resolved/18_elevenreader-session-chronicle-pdf.png` | ✅ Resolved | ElevenReader rendering showing clean, high-fidelity PDF reading of the session chronicle. |
| 19 | `resolved/19_elevenreader-vespers-table-of-chapters.png` | ✅ Resolved | ElevenReader listing the premium Vespers table of chapters and word counts correctly. |
| 20 | `resolved/20_cli-scratchpad-github-paradroidlabs.png` | ✅ Resolved | CLI terminal displaying reassurance scratchpad after GitHub swap to paradroidlabs organization. |
| 21 | `resolved/21_cli-metacognitive-terminal-loop.png` | ✅ Resolved | CLI terminal screenshot capturing the active turn processing the "closing time" request. |
| 22 | `resolved/22_cli-terminal-closing-time.png` | ✅ Resolved | CLI terminal showing the final session closing summary and push confirmations. |
| 23 | `resolved/23_paradroidlabs-complete-desktop-marvel.png` | ✅ Resolved | Ultimate complete workspace desktop showing CLI, Github repository, and ElevenReader audio preview side-by-side. |





