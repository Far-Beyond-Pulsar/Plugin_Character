//! Character (`pulsar.character`): a character as one component.
//!
//! A [`CharacterComponent`] is what a user places and a script drives: one
//! card in the inspector, one entry in the component tree, one component in
//! scripts (`Presentation::Facade`). Placing it places the classes it is
//! built from, each its own component instance nested under it, each its
//! own plugin's: a `SkeletalMeshComponent` (meshes), an `AnimationComponent`
//! (animation), a `CharacterControllerComponent` (physics) and, when the
//! input plugin is enabled, a `PlayerInputComponent`. The plugin links none
//! of their crates: it requires their `class` contracts
//! (`pulsar-plugin.toml`), and reaches the controller by class name and
//! property, as a script does.
//!
//! Each frame its system [`MOVE`] turns the character's `move_input` and
//! `jump` into the controller's desired velocity and vertical speed: after
//! the player's input is read (`pulsar.input.devices`), before physics steps
//! (`pulsar.physics.step`).
//!
//! With the plugin disabled, a level holding characters loads them as
//! unresolved components with their data kept; the members stay their own
//! plugins' live components.
//!
//! This crate is the plugin's root: [`build`] registers everything.

use engine_backend::scene::{ClassSpec, ComponentsBuilder, Transform};
use engine_class_derive::{engine_class, world_component};
use pulsar_world_registry::pulsar_scene_model::attachments::{self, ComponentOwner};
use pulsar_world_registry::systems::COMPONENTS;
use pulsar_world_registry::{Presentation, System, SystemContext};

pulsar_plugins::installed!("pulsar-plugin.toml", build);

/// The class name scripts and level files use.
pub const CHARACTER_CLASS_NAME: &str = "CharacterComponent";

/// The classes a character is built from, placed with it.
pub const MEMBERS: &[&str] = &[
    "SkeletalMeshComponent",
    "AnimationComponent",
    "CharacterControllerComponent",
    "PlayerInputComponent",
];

/// The controller class a character drives.
const CONTROLLER: &str = "CharacterControllerComponent";

/// The camera class that looks through a character.
const CAMERA: &str = "CameraComponent";

/// A character: what moves it, as the player or a script asks.
#[engine_class(category = "Gameplay", clone, debug, serialize, deserialize)]
#[category("Character", category_color = "#F4A261")]
#[category("Control Rotation", category_color = "#40C080")]
#[category("Rotation", category_color = "#5AA0E0")]
#[serde(default)]
pub struct CharacterComponent {
    /// Walking speed, metres per second, at full input.
    #[property(min = 0.0, max = 100.0, step = 0.1, category = "Character")]
    pub walk_speed: f32,
    /// Upward speed a jump starts with, metres per second.
    #[property(min = 0.0, max = 100.0, step = 0.1, category = "Character")]
    pub jump_speed: f32,
    /// Where to walk this frame, in the character's plane (x right, y
    /// forward), each in -1..1: the player's move action or a script sets
    /// it.
    #[property(category = "Character")]
    pub move_input: [f32; 2],
    /// Jump on the next frame the controller is grounded; cleared when it
    /// does.
    #[property(category = "Character")]
    pub jump: bool,
    /// A direction to walk in the world (not the character's own), added by
    /// `add_movement_input` and cleared once the movement system has used it.
    #[property(category = "Character")]
    pub world_input: [f32; 3],

    /// The controller's view direction: yaw in degrees (counter-clockwise seen
    /// from above, as an object's rotation about Y), driven by the look input
    /// (`add_control_yaw`). The camera follows it, movement is walked relative
    /// to its yaw, and the body follows it or not by the options below.
    #[property(category = "Control Rotation")]
    pub control_yaw: f32,
    /// Control pitch in degrees, positive looks up; `add_control_pitch` holds it
    /// between `view_pitch_min` and `view_pitch_max`. Looking up or down never
    /// tilts the walk direction.
    #[property(category = "Control Rotation")]
    pub control_pitch: f32,
    /// Control roll in degrees (0 unless a script sets it).
    #[property(category = "Control Rotation")]
    pub control_roll: f32,
    /// Lowest control pitch, degrees (looking down).
    #[property(min = -89.0, max = 0.0, step = 0.5, category = "Control Rotation")]
    pub view_pitch_min: f32,
    /// Highest control pitch, degrees (looking up).
    #[property(min = 0.0, max = 89.0, step = 0.5, category = "Control Rotation")]
    pub view_pitch_max: f32,
    /// The body's yaw is the control yaw: it turns with the camera and faces
    /// where it looks (first-person and strafing games).
    #[property(category = "Rotation")]
    pub use_controller_rotation_yaw: bool,
    /// The body's pitch is the control pitch.
    #[property(category = "Rotation")]
    pub use_controller_rotation_pitch: bool,
    /// The body's roll is the control roll.
    #[property(category = "Rotation")]
    pub use_controller_rotation_roll: bool,
    /// The body turns, at `rotation_rate`, to face the way it is walking while the
    /// camera keeps the control rotation (third-person free camera). Ignored on
    /// the yaw axis while `use_controller_rotation_yaw` is on.
    #[property(category = "Rotation")]
    pub orient_rotation_to_movement: bool,
    /// How fast the body turns toward its movement, degrees per second.
    #[property(min = 0.0, max = 3600.0, step = 10.0, category = "Rotation")]
    pub rotation_rate: f32,
}

impl Default for CharacterComponent {
    fn default() -> Self {
        Self {
            walk_speed: 4.0,
            jump_speed: 5.0,
            move_input: [0.0, 0.0],
            jump: false,
            world_input: [0.0; 3],
            control_yaw: 0.0,
            control_pitch: 0.0,
            control_roll: 0.0,
            view_pitch_min: -80.0,
            view_pitch_max: 80.0,
            use_controller_rotation_yaw: false,
            use_controller_rotation_pitch: false,
            use_controller_rotation_roll: false,
            orient_rotation_to_movement: true,
            rotation_rate: 540.0,
        }
    }
}

/// A `World` class; [`build`] registers it.
#[world_component]
impl CharacterComponent {}

/// The character's Blueprint and script surface: the controls a graph drives it
/// with (the properties `move_input` is a pair of numbers, which scripts cannot
/// set directly).
#[pulsar_scenedb::component_methods]
impl CharacterComponent {
    /// Walk this way: `x` right and `y` forward, each from -1 to 1 (a stick or
    /// the keys), until it is set again. `0, 0` stands still.
    #[reflect_method(category = "Character")]
    fn set_move(&mut self, x: f32, y: f32) {
        self.move_input = [x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)];
    }

    /// Jump on the next frame the controller is grounded.
    #[reflect_method(category = "Character")]
    fn request_jump(&mut self) {
        self.jump = true;
    }

    /// Turn the control rotation by `degrees` of yaw (positive turns left).
    #[reflect_method(category = "Control Rotation")]
    fn add_control_yaw(&mut self, degrees: f32) {
        if degrees.is_finite() {
            self.control_yaw = wrap_degrees(self.control_yaw + degrees);
        }
    }

    /// Look up (positive) or down by `degrees`, between `view_pitch_min` and
    /// `view_pitch_max`.
    #[reflect_method(category = "Control Rotation")]
    fn add_control_pitch(&mut self, degrees: f32) {
        if degrees.is_finite() {
            let (low, high) = self.pitch_limits();
            self.control_pitch = (self.control_pitch + degrees).clamp(low, high);
        }
    }

    /// Walk toward a direction of the world (x east, y up is ignored, z south)
    /// by `scale`: added to this frame's movement, whatever the character
    /// faces. `set_move` is the relative one (right and forward of the view).
    #[reflect_method(category = "Character")]
    fn add_movement_input(&mut self, x: f32, y: f32, z: f32, scale: f32) {
        if [x, y, z, scale].iter().all(|v| v.is_finite()) {
            self.world_input[0] += x * scale;
            self.world_input[1] += y * scale;
            self.world_input[2] += z * scale;
        }
    }
}

impl CharacterComponent {
    /// The view pitch limits, degrees, finite, inside +-89 and ordered.
    pub fn pitch_limits(&self) -> (f32, f32) {
        let clean = |v: f32, fallback: f32| if v.is_finite() { v.clamp(-89.0, 89.0) } else { fallback };
        let low = clean(self.view_pitch_min, -80.0);
        let high = clean(self.view_pitch_max, 80.0);
        (low.min(high), high.max(low))
    }
}

/// Degrees in (-180, 180].
pub fn wrap_degrees(degrees: f32) -> f32 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

/// The ground-plane direction of `input` (x right, y forward) for a view whose
/// yaw is `yaw_deg`: only the yaw counts, so looking up or down never tilts it.
/// Yaw 0 looks along -Z with +X to the right, as a camera does, and a positive
/// yaw turns left (about +Y). The length is the input's, at most 1.
pub fn movement_direction(yaw_deg: f32, input: [f32; 2]) -> [f32; 3] {
    let yaw = yaw_deg.to_radians();
    let (sin, cos) = yaw.sin_cos();
    let forward = [-sin, -cos];
    let right = [cos, -sin];
    let [x, y] = input;
    let length = (x * x + y * y).sqrt().max(1.0);
    [
        (right[0] * x + forward[0] * y) / length,
        0.0,
        (right[1] * x + forward[1] * y) / length,
    ]
}

/// The yaw (degrees) that faces along ground direction `direction` (the view
/// convention of [`movement_direction`]).
pub fn yaw_facing(direction: [f32; 3]) -> f32 {
    (-direction[0]).atan2(-direction[2]).to_degrees()
}

/// `current` turned toward `target` (degrees) by at most `max_step`, the short way.
pub fn turn_toward(current: f32, target: f32, max_step: f32) -> f32 {
    let difference = wrap_degrees(target - current);
    if difference.abs() <= max_step {
        wrap_degrees(target)
    } else {
        wrap_degrees(current + max_step.copysign(difference))
    }
}

/// The character's movement system.
pub const MOVE: System = System::new("pulsar.character.move", drive)
    .after(&["pulsar.input.devices", COMPONENTS])
    .before(&["pulsar.physics.step"]);

/// Everything the plugin registers: its class (a facade over what it
/// places, with its movement system) and, in the editor, the Create menu's
/// character.
pub fn build(p: &mut pulsar_plugins::PluginBuilder) {
    // The icon the editor shows beside this plugin (Extensions window).
    #[cfg(feature = "editor")]
    p.add(pulsar_plugins::PluginIcon {
        origin: p.origin(),
        format: pulsar_plugins::IconFormat::Png,
        bytes: include_bytes!("../icon.png"),
    });
    p.components().class::<CharacterComponent>(ClassSpec {
        requires: MEMBERS,
        systems: &[MOVE],
        presentation: Presentation::Facade,
        ..ClassSpec::default()
    });
    #[cfg(feature = "editor")]
    editor::build(p);
}

/// One character's step, read before writing (the world is borrowed by the
/// query while reading).
struct Step {
    owner: pulsar_scenedb::Entity,
    instance: pulsar_scenedb::Entity,
    velocity: [f32; 3],
    jump: Option<f32>,
    /// The ground direction walked this frame (unit or shorter).
    direction: [f32; 3],
    control: [f32; 3],
    pitch_yaw_roll_follow: [bool; 3],
    orient: bool,
    rate: f32,
}

/// [`MOVE`]: every enabled character sets its controller's desired velocity
/// from `move_input` (in the owner's facing plane, `walk_speed` at full
/// input) and starts a jump when asked and grounded.
fn drive(ctx: &mut SystemContext<'_>) {
    let world = &mut *ctx.world;
    let steps: Vec<Step> = world
        .query::<(&CharacterComponent, &ComponentOwner)>()
        .filter(|(_, (_, link))| link.is_enabled())
        .map(|(instance, (character, link))| {
            // Right and forward of the view's yaw, plus what was asked of the world.
            let mut direction = movement_direction(character.control_yaw, character.move_input);
            for axis in [0, 2] {
                direction[axis] += character.world_input[axis];
            }
            let length = (direction[0] * direction[0] + direction[2] * direction[2])
                .sqrt()
                .max(1.0);
            direction = [direction[0] / length, 0.0, direction[2] / length];
            Step {
                owner: link.entity(),
                instance,
                velocity: [
                    direction[0] * character.walk_speed,
                    0.0,
                    direction[2] * character.walk_speed,
                ],
                jump: character.jump.then_some(character.jump_speed),
                direction,
                control: [character.control_pitch, character.control_yaw, character.control_roll],
                pitch_yaw_roll_follow: [
                    character.use_controller_rotation_pitch,
                    character.use_controller_rotation_yaw,
                    character.use_controller_rotation_roll,
                ],
                orient: character.orient_rotation_to_movement,
                rate: character.rotation_rate.max(0.0),
            }
        })
        .collect();
    let seconds = ctx.delta_seconds;
    for step in steps {
        // The body's own rotation: it follows the control rotation on the axes
        // that ask, and turns toward its movement otherwise.
        // Read first and write only a change: a write marks the transform
        // moved, and the physics step would take the body's position from it.
        let current = world.get::<Transform>(step.owner).map(|t| t.rotation);
        if let Some(current) = current {
            let [mut pitch, mut yaw, mut roll] = current;
            let [follow_pitch, follow_yaw, follow_roll] = step.pitch_yaw_roll_follow;
            if follow_pitch {
                pitch = step.control[0];
            }
            if follow_roll {
                roll = step.control[2];
            }
            if follow_yaw {
                yaw = step.control[1];
            } else if step.orient && step.direction[0].hypot(step.direction[2]) > 0.01 {
                yaw = turn_toward(yaw, yaw_facing(step.direction), step.rate * seconds);
            }
            if [pitch, yaw, roll] != current {
                if let Some(mut transform) = world.get_mut::<Transform>(step.owner) {
                    transform.rotation = [pitch, yaw, roll];
                }
            }
        }
        // The camera of this object follows the control rotation.
        if let Some(camera) = class_index(world, step.owner, CAMERA) {
            let _ = pulsar_world_registry::set_component_property(
                world,
                step.owner,
                CAMERA,
                camera,
                "control_yaw_deg",
                serde_json::json!(step.control[1]),
            );
            let follows = pulsar_world_registry::get_component_property(
                world,
                step.owner,
                CAMERA,
                camera,
                "use_pawn_control_rotation",
            )
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
            if follows {
                let _ = pulsar_world_registry::set_component_property(
                    world,
                    step.owner,
                    CAMERA,
                    camera,
                    "pitch_deg",
                    serde_json::json!(step.control[0]),
                );
            }
        }
        if let Some(mut character) = world.get_mut::<CharacterComponent>(step.instance) {
            character.world_input = [0.0; 3];
        }
        let Some(controller) = controller_index(world, step.owner) else {
            continue;
        };
        let set = |world: &mut pulsar_scenedb::World, property: &str, value: serde_json::Value| {
            if let Err(error) = pulsar_world_registry::set_component_property(
                world, step.owner, CONTROLLER, controller, property, value,
            ) {
                tracing::warn!("character: {CONTROLLER}.{property}: {error}");
            }
        };
        set(world, "desired_velocity", serde_json::json!(step.velocity));
        if let Some(speed) = step.jump {
            let grounded = pulsar_world_registry::get_component_property(
                world, step.owner, CONTROLLER, controller, "grounded",
            )
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
            if grounded {
                set(world, "vertical_velocity", serde_json::json!(speed));
                if let Some(mut character) = world.get_mut::<CharacterComponent>(step.instance) {
                    character.jump = false;
                }
            }
        }
    }
}

/// The index, among the owner's instances of the controller class, of the
/// one the character drives (its first).
fn class_index(
    world: &pulsar_scenedb::World,
    owner: pulsar_scenedb::Entity,
    class: &str,
) -> Option<u32> {
    attachments::instances(world, owner)
        .iter()
        .any(|instance| {
            attachments::meta(world, *instance).is_some_and(|meta| meta.class_name == class)
        })
        .then_some(0)
}

fn controller_index(world: &pulsar_scenedb::World, owner: pulsar_scenedb::Entity) -> Option<u32> {
    attachments::instances(world, owner)
        .iter()
        .any(|instance| {
            attachments::meta(world, *instance).is_some_and(|meta| meta.class_name == CONTROLLER)
        })
        .then_some(0)
}

#[cfg(feature = "editor")]
mod editor {
    use ui_level_editor::commands::{CommandResult, TypedComponent};
    use ui_level_editor::extend::{add_object, CreateEntry};
    use ui_level_editor::scene_edit::{ObjectType, Transform};
    use ui_level_editor::LevelEditorState;

    /// The Create menu's character: an object with a character, which brings
    /// its skeletal mesh, animator, controller and input with it.
    fn add_character(state: &mut LevelEditorState) -> CommandResult {
        add_object(
            state,
            "Character",
            ObjectType::Empty,
            Transform::default(),
            vec![TypedComponent::new(super::CharacterComponent::default())],
        )
    }

    pub(super) fn build(p: &mut pulsar_plugins::PluginBuilder) {
        p.add(CreateEntry {
            origin: p.origin(),
            id: "character",
            menu_label: Some(|| "Character".to_string()),
            environment_label: None,
            quick_add: None,
            order: 50,
            offered: None,
            create: add_character,
        });
    }
}

// The plugin as a library of its own, loaded by the editor from `plugins/editor`
// (`cargo build -p <this package> --release --features dll`).
#[cfg(feature = "dll")]
plugin_editor_api::export_library_plugin!();

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 3], b: [f32; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-5, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn forward_follows_the_yaw_and_right_is_beside_it() {
        near(movement_direction(0.0, [0.0, 1.0]), [0.0, 0.0, -1.0]);
        near(movement_direction(0.0, [1.0, 0.0]), [1.0, 0.0, 0.0]);
        // A quarter turn left: forward is -X, right is -Z.
        near(movement_direction(90.0, [0.0, 1.0]), [-1.0, 0.0, 0.0]);
        near(movement_direction(90.0, [1.0, 0.0]), [0.0, 0.0, -1.0]);
        near(movement_direction(180.0, [0.0, 1.0]), [0.0, 0.0, 1.0]);
        near(movement_direction(-90.0, [0.0, 1.0]), [1.0, 0.0, 0.0]);
        // A diagonal is not faster than straight.
        let d = movement_direction(30.0, [1.0, 1.0]);
        assert!((d[0].hypot(d[2]) - 1.0).abs() < 1e-5);
        // Less than full input walks less.
        let half = movement_direction(30.0, [0.0, 0.5]);
        assert!((half[0].hypot(half[2]) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn facing_a_direction_is_the_yaw_that_walks_it() {
        for yaw in [-170.0_f32, -90.0, -10.0, 0.0, 45.0, 135.0] {
            let walked = movement_direction(yaw, [0.0, 1.0]);
            assert!(wrap_degrees(yaw_facing(walked) - yaw).abs() < 1e-3, "yaw {yaw}");
        }
    }

    #[test]
    fn turning_toward_goes_the_short_way_at_the_rate() {
        assert_eq!(turn_toward(0.0, 90.0, 10.0), 10.0);
        assert_eq!(turn_toward(0.0, -90.0, 10.0), -10.0);
        assert_eq!(turn_toward(170.0, -170.0, 10.0), 180.0, "across the seam, 20 degrees apart");
        assert_eq!(turn_toward(0.0, 5.0, 10.0), 5.0, "arrives without overshooting");
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(wrap_degrees(-180.0), 180.0);
    }

    #[test]
    fn control_pitch_holds_its_limits_and_yaw_wraps() {
        let mut c = CharacterComponent::default();
        c.add_control_pitch(500.0);
        assert_eq!(c.control_pitch, 80.0);
        c.add_control_pitch(-1000.0);
        assert_eq!(c.control_pitch, -80.0);
        c.view_pitch_max = 30.0;
        c.add_control_pitch(500.0);
        assert_eq!(c.control_pitch, 30.0);
        c.add_control_yaw(200.0);
        c.add_control_yaw(200.0);
        assert_eq!(c.control_yaw, wrap_degrees(400.0));
        c.add_movement_input(1.0, 0.0, 0.0, 0.5);
        c.add_movement_input(1.0, 0.0, 0.0, 0.5);
        assert_eq!(c.world_input, [1.0, 0.0, 0.0]);
    }
}
