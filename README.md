# Plugin_Character

A character as one component: a facade over a skeletal mesh, its animator, a character controller and the player's input, with the movement system that drives them.

`pulsar.character` ("Character") is a plugin of
[Pulsar-Native](https://github.com/Far-Beyond-Pulsar/Pulsar-Native), the engine and editor that renders with
[Helio](https://github.com/Far-Beyond-Pulsar/Helio). Pulsar-Native vendors this repository as a git submodule
at `plugins/vendor/character`.

## What it owns

Every object kind is a plugin. A plugin owns its SceneDB components (their
data and scriptable methods; the editor calls the same methods scripts do),
the contracts other kinds read, its file formats and its editor UI. Other
plugins meet this one only through contracts, and this one meets them the
same way.

| Crate | Path | What |
|---|---|---|
| `character_plugin` | `.` | Character (`pulsar.character`): the plugin's root crate. It holds the `CharacterComponent` class, its movement system `pulsar.character.move`, and the level editor's Create entry (with `editor`) |

`CharacterComponent` is what a user places and a script drives. In the
inspector it is one card, in the component tree one entry, and in scripts one
component (`Presentation::Facade`). Placing it also places the classes it is
built from (`ClassSpec.requires`). Each of those is its own component
instance nested under it, owned by its own plugin:

- `SkeletalMeshComponent` (meshes)
- `AnimationComponent` (animation)
- `CharacterControllerComponent` (physics)
- `PlayerInputComponent` (input), when that plugin is enabled

The plugin links none of their crates. It requires their `class` contracts,
and reaches the controller by class name and property, as a script does.

Each frame the `pulsar.character.move` system turns the character's
`move_input` and `jump` into the controller's desired velocity and vertical
speed. It runs after the player's input is read (`pulsar.input.devices`) and
before physics steps (`pulsar.physics.step`).

With the plugin disabled, a level holding characters still loads. Each
character is kept as an unresolved component with its data intact, and its
member components stay live under their own plugins.

- **Runtime module** (what a game built for a project links): `character_plugin`.
- **Editor module** (what the editor links in addition): `character_plugin` with its
  `editor` feature (the Create menu's "Character").
- **Requires**: the class contracts `pulsar.meshes.classes`,
  `pulsar.animation.classes` and `pulsar.physics.classes`, plus
  `pulsar.input.classes` (optional). The descriptor is `pulsar-plugin.toml`.
  If a required provider is absent or disabled, activation names it and
  disables this plugin.
- **Provides**: `pulsar.character.classes` (`CharacterComponent`).

## Building and testing

Like [Plugin_Physics](https://github.com/Far-Beyond-Pulsar/Plugin_Physics), this plugin's crate is a member
of Pulsar-Native's workspace. Its dependencies are `workspace = true`, so it
builds only inside a Pulsar-Native checkout:

```bash
git clone --recursive https://github.com/Far-Beyond-Pulsar/Pulsar-Native
cd Pulsar-Native
cargo check -p character_plugin --features character_plugin/editor --tests
cargo test -p installed_plugins --test character
```

The character's tests compose it with the plugins it is built from, so they
live in Pulsar-Native's installed set (`plugins/installed/tests/character.rs`):
placing it, absent input, a refused member, walking in a game, and reloading
with the plugin removed. This repository's CI checks the plugin inside a
Pulsar-Native checkout: rustfmt, clippy and the `ci` profile's tests.

A change that spans this plugin and Pulsar-Native merges dependency-first:
merge here, re-pin `plugins/vendor/character` in Pulsar-Native to this
repository's `main`, then merge the Pulsar-Native PR (Pulsar-Native's
`.agents/CRATES.md`, "Submodule pins").

## Architecture

- [Every object kind is a plugin](https://github.com/Far-Beyond-Pulsar/Helio/blob/main/docs/architecture/plugin-migration.md): the
  rule, the plugin table and the "As built" notes of each phase.
- [The `helio-architecture` skill](https://github.com/Far-Beyond-Pulsar/Helio/blob/main/.agents/skills/helio-architecture/SKILL.md): required reading for any change
  here (see `AGENTS.md`).
- Pulsar-Native's `.agents/PLUGIN_SYSTEM.md` ("Contracts", "Component
  classes") and `.agents/PLUGIN_API_REFACTOR_PLAN.md` section 14: plugin
  descriptors, class specs, systems and facades.

The plugin was written in Pulsar-Native's p22 batch (Pulsar-Native#1608
phase 5) and moved here before that batch merged.

## License

MIT (`LICENSE`).
