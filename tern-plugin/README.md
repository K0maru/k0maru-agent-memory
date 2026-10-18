# K0maru Agent Memory for Tern

Native [Tern](https://github.com/tern-tool/tern) plugin that brings the
K0maru Agent Memory hub into the terminal: a Living ADR Guard dashboard, a
`k0maru doctor` health view, structured rendering of CLI output, and a Carly
`recall` export for conversational memory search.

The plugin shells out to the `k0maru` binary, so the CLI must be on `PATH`
(`cargo install --path .` or the installer described in the
[main README](../README.md)). Search and guard rules come from the active
Markdown vault (`--vault`, or the `K0MARU_VAULT` / current directory
resolution used by the CLI).

## Installation

Install from the repository:

```sh
tern plugin install github.com/K0maru/k0maru-agent-memory/tern-plugin
```

For development, link this checkout instead (edits reload without
reinstalling):

```sh
tern plugin link ./tern-plugin
```

After changing the plugin, regenerate the Luau type definitions used by
editors and `luau-lsp`:

```sh
tern plugin types tern-plugin
```

## Features

### Memory Cockpit block

Open it from the palette (`K0maru: Open Memory Cockpit`) or with the action
`plugin.k0maru-memory.cockpit`; the block opens beside the focused pane. The
action can be bound to a chord in Tern's keymap settings.

The cockpit runs two probes in the block's working directory:

| Probe | Command | Rendered as |
|---|---|---|
| Doctor | `k0maru doctor --json` | Per-check table (`Category`, `Check`, `Status`, `Detail`) plus pass/warn/fail counts and a health meter |
| Guard | `k0maru guard check --json` | PASS / violation badge, checked file and hunk counts, and a diagnostic card per violation |

Press `r` to re-run both probes. Guard violations render as Tern
diagnostics with the rule id, matched content, source note, remediation, and
a link to the offending file and line.

### CLI lens

Commands matching `k0maru *` or `k0maru-memory *` get a structured view
instead of raw text:

- `k0maru doctor --json` renders the test summary and the diagnostic table.
- `k0maru guard check --json` renders the pass/fail badge, counts, duration,
  and each violation as a diagnostic card. This works even when the guard
  exits non-zero, which is its normal behavior when a deny rule fires.
- Any other JSON output falls back to Tern's collapsible JSON tree, and
  non-JSON output keeps the raw terminal text.

### Palette commands

| Command | Description |
|---|---|
| `K0maru: Open Memory Cockpit` | Opens the cockpit block beside the focused pane |
| `K0maru: Run Living ADR Guard Check` | Runs `k0maru guard check --json` in the focused pane's directory and reports PASS, warnings, or violations in a toast |

Both commands are also available as actions
`plugin.k0maru-memory.cockpit` and `plugin.k0maru-memory.guard_check`.

### Carly `recall` export

Registered with:

```lua
tern.carly.export("recall", { sig = "(query: string) -> any", doc = "..." }, fn)
```

Carly (or any plugin sharing the window) can call it:

```lua
local results = await(plugins["k0maru-memory"].recall("why did we choose Markdown storage?"))
```

`recall` runs `k0maru search <query> --json` in the focused pane's directory
and replies with the decoded JSON array of hybrid search hits
(`path`, `title`, `score`, `snippet`, ranks, graph boost). It answers as an
empty array when nothing matched and fails the call when the search errored
or timed out (15 s reply budget).

## Files

| File | Purpose |
|---|---|
| `plugin.toml` | Manifest: block `cockpit`, lens `cli`, palette registration |
| `host.luau` | `tern.block.define("cockpit", ...)` and `tern.lens.define("cli", ...)` |
| `window.luau` | `tern.command` palette commands and the Carly `recall` export |
| `tern.d.luau` | Generated Luau type definitions (`tern plugin types`); do not edit by hand |

## Manifest

```toml
schema = 1
id = "k0maru-memory"
name = "K0maru Agent Memory"
version = "0.12.0"
host = "host.luau"
window = "window.luau"

[[blocks]]
id = "cockpit"
title = "K0maru Memory Cockpit"
palette = true

[[lenses]]
id = "cli"
match = ["k0maru *", "k0maru-memory *"]
```

## Requirements

- Tern with plugin support (host blocks, lenses, palette commands, Carly
  exports).
- `k0maru` 0.12.0 or newer on `PATH`.
- A Markdown vault with Living ADR invariants for the Guard view; without
  one, the guard reports zero rules and passes.

## Uninstall

```sh
tern plugin remove k0maru-memory
```
