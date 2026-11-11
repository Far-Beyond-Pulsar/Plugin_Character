# Plugin_Character

`pulsar.character`: a character as one component, a facade over a skeletal mesh, its animator, a character controller and the player's input, with its movement system.
Pulsar-Native vendors this repository at `plugins/vendor/character`. It builds only
inside a Pulsar-Native checkout (see `README.md`).

## Required for every agent

Before reading, designing, editing, reviewing or briefing any change here,
load the **`helio-architecture`** skill. Its single source is Helio's
[`.agents/skills/helio-architecture/SKILL.md`](https://github.com/Far-Beyond-Pulsar/Helio/blob/main/.agents/skills/helio-architecture/SKILL.md). In a Pulsar-Native
checkout it is at `crates/renderer/helio/.agents/skills/helio-architecture/SKILL.md`.
That applies to bug fixes, perf work, tests and small changes too.

Follow the skill's steps in order and end with its report section (Design,
AAA comparison, Deleted, Verified). Anything you brief (a subagent, another
session or another agent) must load the skill too. The skill is the single
source of these rules: this file only points to it, so change the skill
rather than copying rules here.

## Docs

| File | What |
|---|---|
| [Helio `docs/architecture/plugin-migration.md`](https://github.com/Far-Beyond-Pulsar/Helio/blob/main/docs/architecture/plugin-migration.md) | Every object kind is a plugin; the plugin table; "As built" notes per phase |
| Pulsar-Native `AGENTS.md`, `.agents/PLUGIN_SYSTEM.md`, `.agents/CRATES.md` | Plugin descriptors, contracts, component classes, the installed set, submodule pins |
| Pulsar-Native `.agents/PLUGIN_API_REFACTOR_PLAN.md` section 14 | Class specs, systems, facades as built (p22) |

## Key commands (from the Pulsar-Native root)

```
cargo check -p character_plugin --features character_plugin/editor --tests
cargo test -p installed_plugins --test character   placing, playing, reloading without the plugin
cargo test -p one_owner                            every concept has one owner (scans this plugin)
cargo test -p installed_plugins                    the installed set and absent plugins
```
