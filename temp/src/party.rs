//! Screen-aligned color and upright, animated shadow geometry share a foot pivot.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageLoaderSettings, ImageSampler},
    pbr::{ExtendedMaterial, MaterialExtension, NotShadowCaster},
    prelude::*,
    render::{
        mesh::{Indices, MeshTag},
        render_resource::{AsBindGroup, PrimitiveTopology, ShaderRef},
        view::RenderLayers,
    },
    transform::TransformSystem,
};

use crate::{
    billboard::{BODY_LAYER, ScreenAligned},
    world::{PLAN, floor_height, ground, walkable},
};

pub struct PartyPlugin;

/// Shared movement state for camera, interaction and HUD systems.
#[derive(Resource)]
pub struct Expedition {
    pub leader: Vec2,
    pub walking: bool,
    pub quickstepping: bool,
    trail: [Vec2; TRAIL_CAPACITY],
    newest: usize,
}

const TRAIL_CAPACITY: usize = 256;
const TRAIL_STEP: f32 = 0.06;
const FOLLOW_DISTANCE: [f32; 4] = [0.0, 1.65, 3.30, 4.95];
const STAGGER: [f32; 4] = [0.0, -0.28, 0.28, -0.18];
const CHARACTER_HEIGHT: f32 = 2.02;
const PIXEL_HEIGHT: [f32; 4] = [55.0, 62.0, 55.0, 56.0];
const WALK_SPEED: f32 = 2.7;
const QUICKSTEP_SPEED: f32 = 4.8;

impl Default for Expedition {
    fn default() -> Self {
        let leader = Vec2::from_array(PLAN.spawn);
        let mut trail = [Vec2::ZERO; TRAIL_CAPACITY];
        for age in 0..TRAIL_CAPACITY {
            trail[(TRAIL_CAPACITY - age) % TRAIL_CAPACITY] =
                leader + Vec2::Y * age as f32 * TRAIL_STEP;
        }
        Self {
            leader,
            walking: false,
            quickstepping: false,
            trail,
            newest: 0,
        }
    }
}

impl Expedition {
    fn record(&mut self, position: Vec2) {
        let previous = self.trail[self.newest];
        let delta = position - previous;
        let length = delta.length();
        if length >= TRAIL_STEP {
            let direction = delta / length;
            for index in 1..=(length / TRAIL_STEP) as usize {
                self.newest = (self.newest + 1) % TRAIL_CAPACITY;
                self.trail[self.newest] = previous + direction * (index as f32 * TRAIL_STEP);
            }
        }
    }

    fn position_behind(&self, distance: f32) -> Vec2 {
        let newest = self.trail[self.newest];
        let remainder = self.leader.distance(newest);
        if distance <= remainder && remainder > 0.0 {
            return self.leader.lerp(newest, distance / remainder);
        }
        let offset = ((distance - remainder) / TRAIL_STEP).max(0.0);
        let age = (offset as usize).min(TRAIL_CAPACITY - 2);
        let i = (self.newest + TRAIL_CAPACITY - age) % TRAIL_CAPACITY;
        self.trail[i].lerp(
            self.trail[(i + TRAIL_CAPACITY - 1) % TRAIL_CAPACITY],
            offset.fract(),
        )
    }
}

/// Sample expedition state after this set for same-frame tracking.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartyMovement;

#[derive(Component)]
struct Traveler {
    ordinal: usize,
    position: Vec2,
    heading: Vec2,
    stride: f32,
    moving: bool,
    displayed: usize,
    caster: Entity,
    camera_opacity: f32,
}

#[derive(Component)]
struct BodyCaster;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct StandingSurface {
    #[texture(100)]
    #[sampler(101)]
    shape: Handle<Image>,
    #[uniform(102)]
    body_height: f32,
}
impl MaterialExtension for StandingSurface {
    fn fragment_shader() -> ShaderRef {
        "party/costume.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        Self::fragment_shader()
    }
}
type Costume = ExtendedMaterial<StandardMaterial, StandingSurface>;

#[derive(Resource)]
struct Frames {
    sprites: Vec<Handle<Mesh>>,
    bodies: Vec<Handle<Mesh>>,
}

impl Plugin for PartyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<Costume>::default())
            .init_resource::<Expedition>()
            .add_systems(Startup, spawn_party)
            .add_systems(Update, move_party.in_set(PartyMovement))
            .add_systems(
                PostUpdate,
                animate_party.before(TransformSystem::TransformPropagate),
            );
    }
}

fn sprite_frame(row: usize, column: usize) -> Mesh {
    let texel = CHARACTER_HEIGHT / PIXEL_HEIGHT[row / 4];
    let left = -48.0 * texel * 0.5;
    let bottom = -2.0 * texel;
    let top = 62.0 * texel;
    let u0 = column as f32 / 8.0;
    let u1 = (column + 1) as f32 / 8.0;
    let v0 = row as f32 / 16.0;
    let v1 = (row + 1) as f32 / 16.0;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [left, bottom, 0.0],
            [-left, bottom, 0.0],
            [-left, top, 0.0],
            [left, top, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_1,
        vec![[left, bottom], [-left, bottom], [-left, top], [left, top]],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn spawn_party(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut costumes: ResMut<Assets<Costume>>,
    expedition: Res<Expedition>,
) {
    let atlas: Handle<Image> = asset_server.load_with_settings(
        "party/travelers.png",
        |settings: &mut ImageLoaderSettings| settings.sampler = ImageSampler::nearest(),
    );
    let shape = asset_server.load_with_settings(
        "party/body-surface.png",
        |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.sampler = ImageSampler::nearest();
        },
    );
    let costume = Costume {
        base: StandardMaterial {
            base_color_texture: Some(atlas),
            alpha_mode: AlphaMode::Mask(0.5),
            perceptual_roughness: 1.0,
            reflectance: 0.16,
            ..default()
        },
        extension: StandingSurface {
            shape,
            body_height: CHARACTER_HEIGHT,
        },
    };
    let body_material = materials.add(StandardMaterial::default());
    let bodies: Vec<Handle<Mesh>> = (0..32)
        .map(|mesh| {
            asset_server.load(
                GltfAssetLabel::Primitive { mesh, primitive: 0 }.from_asset("party/bodies.glb"),
            )
        })
        .collect();
    let mut frames = Vec::with_capacity(128);
    for row in 0..16 {
        for column in 0..8 {
            frames.push(meshes.add(sprite_frame(row, column)));
        }
    }
    let names = [
        "Aurelia · amber sword-bearer",
        "Orin · stargazer",
        "Seren · silver acolyte",
        "Bram · bronze warden",
    ];
    for ordinal in 0..4 {
        let mut position = expedition.position_behind(FOLLOW_DISTANCE[ordinal]);
        position.x += STAGGER[ordinal];
        let frame = ordinal * 32;
        let caster = commands
            .spawn((
                BodyCaster,
                Mesh3d(bodies[ordinal * 8].clone()),
                MeshMaterial3d(body_material.clone()),
                RenderLayers::layer(BODY_LAYER),
                Transform::from_translation(ground(position))
                    .with_scale(Vec3::splat(CHARACTER_HEIGHT)),
            ))
            .id();
        commands.spawn((
            Name::new(names[ordinal]),
            Traveler {
                ordinal,
                position,
                heading: Vec2::NEG_Y,
                stride: ordinal as f32 * 1.5,
                moving: false,
                displayed: frame,
                caster,
                camera_opacity: 1.0,
            },
            Mesh3d(frames[frame].clone()),
            MeshMaterial3d(costumes.add(costume.clone())),
            MeshTag::default(),
            ScreenAligned,
            NotShadowCaster,
            Transform::from_translation(ground(position)),
        ));
    }
    commands.insert_resource(Frames {
        sprites: frames,
        bodies,
    });
}

fn clear_feet(from: Vec2, position: Vec2) -> bool {
    walkable(position)
        && (floor_height(position) - floor_height(from)).abs() < 0.28
        && walkable(position + Vec2::new(-0.17, 0.0))
        && walkable(position + Vec2::new(0.17, 0.0))
        && walkable(position + Vec2::new(0.0, -0.13))
        && walkable(position + Vec2::new(0.0, 0.13))
}

fn move_party(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    cameras: Query<&Transform, (With<Camera3d>, Without<Traveler>)>,
    mut expedition: ResMut<Expedition>,
    mut travelers: Query<&mut Traveler>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let right = camera.right();
    let forward = camera.forward();
    let right = Vec2::new(right.x, right.z).normalize_or_zero();
    let forward = Vec2::new(forward.x, forward.z).normalize_or_zero();
    let horizontal = i32::from(keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight))
        - i32::from(keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft));
    let vertical = i32::from(keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp))
        - i32::from(keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown));
    let direction = (right * horizontal as f32 + forward * vertical as f32).normalize_or_zero();
    let fast = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = if fast { QUICKSTEP_SPEED } else { WALK_SPEED };
    let dt = time.delta_secs();
    let movement = direction * speed * dt;
    let before = expedition.leader;
    if movement != Vec2::ZERO {
        let steps = (movement.length() / 0.055).ceil().max(1.0) as usize;
        let step = movement / steps as f32;
        for _ in 0..steps {
            let candidate = expedition.leader + step;
            if clear_feet(expedition.leader, candidate) {
                expedition.leader = candidate;
            } else {
                let horizontal = expedition.leader + Vec2::new(step.x, 0.0);
                if step.x != 0.0 && clear_feet(expedition.leader, horizontal) {
                    expedition.leader = horizontal;
                }
                let vertical = expedition.leader + Vec2::new(0.0, step.y);
                if step.y != 0.0 && clear_feet(expedition.leader, vertical) {
                    expedition.leader = vertical;
                }
            }
            let position = expedition.leader;
            expedition.record(position);
        }
    }
    expedition.walking = expedition.leader.distance_squared(before) > 0.000001;
    expedition.quickstepping = expedition.walking && fast;
    let mut occupied = [Vec2::ZERO; 4];
    for traveler in &travelers {
        occupied[traveler.ordinal] = traveler.position;
    }
    for mut traveler in &mut travelers {
        let ordinal = traveler.ordinal;
        let mut next = expedition.leader;
        if ordinal > 0 {
            let distance = FOLLOW_DISTANCE[ordinal];
            let center = expedition.position_behind(distance);
            let tangent = (expedition.position_behind(distance - 0.18)
                - expedition.position_behind(distance + 0.18))
            .normalize_or_zero();
            let side = Vec2::new(-tangent.y, tangent.x);
            next = center + side * STAGGER[ordinal];
            if !clear_feet(traveler.position, next) {
                next = center;
            }
            // A small passing lane keeps a retraced path readable on reversals.
            for (other, position) in occupied.iter().enumerate() {
                if other != ordinal && next.distance_squared(*position) < 0.46 * 0.46 {
                    let passing = next + side * if ordinal % 2 == 0 { 0.38 } else { -0.38 };
                    if clear_feet(traveler.position, passing) {
                        next = passing;
                    }
                }
            }
            let step = (next - traveler.position).clamp_length_max(speed * dt * 1.22);
            next = traveler.position + step;
            if !clear_feet(traveler.position, next) {
                next = traveler.position;
            }
        }
        let delta = next - traveler.position;
        let distance = delta.length();
        traveler.moving = distance > 0.001;
        if traveler.moving {
            traveler.heading = delta / distance;
            traveler.stride = (traveler.stride + distance / 0.16) % 6.0;
            traveler.position = next;
        }
    }
}
/// The atlas is drawn from four fixed sides of the shadow body. Between two
/// pose changes its depth samples stay on that body even as the camera orbits.
fn costume_view(heading: Vec2, camera_right: Vec2) -> (usize, MeshTag) {
    let body_right = Vec2::new(heading.y, -heading.x);
    let across = heading.dot(camera_right);
    let toward = body_right.dot(camera_right);
    let (row, art_right) = if across.abs() > toward.abs() {
        if across > 0.0 {
            (2, heading)
        } else {
            (3, -heading)
        }
    } else if toward > 0.0 {
        (1, body_right)
    } else {
        (0, -body_right)
    };
    // World XZ of the painting's right axis, for WGSL unpack2x16snorm.
    let [x, z] = (art_right * f32::from(i16::MAX)).round().to_array();
    (
        row,
        MeshTag(u32::from(x as i16 as u16) | (u32::from(z as i16 as u16) << 16)),
    )
}

fn animate_party(
    time: Res<Time>,
    frames: Res<Frames>,
    expedition: Res<Expedition>,
    mut costumes: ResMut<Assets<Costume>>,
    cameras: Query<&Transform, (With<Camera3d>, Without<Traveler>, Without<BodyCaster>)>,
    mut travelers: Query<
        (
            &mut Traveler,
            &mut Transform,
            &mut Mesh3d,
            &mut MeshTag,
            &MeshMaterial3d<Costume>,
            &mut Visibility,
        ),
        Without<BodyCaster>,
    >,
    mut bodies: Query<(&mut Transform, &mut Mesh3d, &mut Visibility), With<BodyCaster>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    // Facing selection uses the view's horizontal axes, not each actor's
    // eye-to-foot vector. All actors share the same screen right/back.
    let camera_right = camera.right();
    let right = Vec2::new(camera_right.x, camera_right.z).normalize_or_zero();
    let leader_foot = ground(expedition.leader);
    let forward = *camera.forward();
    let up = *camera.up();
    let pitch = (-forward.y).clamp(-1.0, 1.0).asin().to_degrees();
    let t = ((22.0 - pitch) / 14.0).clamp(0.0, 1.0);
    let close_clearance = t * t * (3.0 - 2.0 * t);
    let leader_delta = leader_foot - camera.translation;
    let leader_depth = leader_delta.dot(forward).max(0.1);
    let leader_x = leader_delta.dot(*camera_right) / leader_depth;
    let leader_y = (leader_delta.dot(up) + CHARACTER_HEIGHT * 0.5) / leader_depth;
    for (mut traveler, mut transform, mut mesh, mut tag, material, mut visibility) in &mut travelers {
        let foot = ground(traveler.position);
        transform.translation = foot;
        // Preserve the real trail and collision positions. Only a companion
        // between the camera and leader, whose projected body overlaps the
        // leader, fades away for the last part of the eye-level dolly.
        let delta = foot - camera.translation;
        let depth = delta.dot(forward);
        let in_front = depth > 0.1 && depth < leader_depth - 0.15;
        let overlaps = in_front
            && (delta.dot(*camera_right) / depth - leader_x).abs()
                < 0.75 / depth + 0.65 / leader_depth
            && ((delta.dot(up) + CHARACTER_HEIGHT * 0.5) / depth - leader_y).abs()
                < CHARACTER_HEIGHT * 0.55 * (1.0 / depth + 1.0 / leader_depth);
        let desired_opacity = if traveler.ordinal > 0 && overlaps {
            1.0 - close_clearance
        } else {
            1.0
        };
        let previous_opacity = traveler.camera_opacity;
        let mut opacity = previous_opacity.lerp(
            desired_opacity,
            1.0 - (-10.0 * time.delta_secs()).exp(),
        );
        if (opacity - desired_opacity).abs() < 0.001 {
            opacity = desired_opacity;
        }
        traveler.camera_opacity = opacity;
        if opacity != previous_opacity {
            if let Some(costume) = costumes.get_mut(&material.0) {
                costume.base.base_color.set_alpha(opacity);
                costume.base.alpha_mode = if opacity >= 0.999 {
                    AlphaMode::Mask(0.5)
                } else {
                    AlphaMode::Blend
                };
            }
        }
        let shown = if opacity <= 0.001 { Visibility::Hidden } else { Visibility::Inherited };
        visibility.set_if_neq(shown);
        let (direction, art_axes) = costume_view(traveler.heading, right);
        tag.set_if_neq(art_axes);
        // Feet stay at the physical floor; gait and breathing are pixel animation.
        let frame = if traveler.moving {
            2 + traveler.stride as usize
        } else {
            ((time.elapsed_secs() * 1.6 + traveler.ordinal as f32 * 0.7) as usize) % 2
        };
        let (mut body, mut caster_mesh, mut body_visibility) = bodies
            .get_mut(traveler.caster)
            .expect("traveler has a caster");
        body.translation = foot;
        body.rotation = Quat::from_rotation_y(traveler.heading.x.atan2(traveler.heading.y));
        // Shadow presentation retreats with the painted companion; no invisible
        // full-size body remains between the leader and the morning light.
        body.scale = Vec3::splat(CHARACTER_HEIGHT * opacity.max(0.001));
        body_visibility.set_if_neq(shown);
        if traveler.displayed % 8 != frame {
            caster_mesh.0 = frames.bodies[traveler.ordinal * 8 + frame].clone();
        }
        let index = (traveler.ordinal * 4 + direction) * 8 + frame;
        if traveler.displayed != index {
            mesh.0 = frames.sprites[index].clone();
            traveler.displayed = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_keeps_each_painted_surface_on_its_side_of_the_shadow_body() {
        let heading = Vec2::new(0.6, -0.8);
        let side = Vec2::new(heading.y, -heading.x);
        for (row, painted_right) in [(0, -side), (1, side), (2, heading), (3, -heading)] {
            for yaw in [-0.76, 0.0, 0.76] {
                let (view, tag) =
                    costume_view(heading, Vec2::from_angle(yaw).rotate(painted_right));
                let decoded = Vec2::new((tag.0 as i16) as f32, ((tag.0 >> 16) as i16) as f32)
                    / f32::from(i16::MAX);
                assert_eq!(view, row);
                assert!(decoded.distance(painted_right) < 0.00003);
            }
        }
    }
}
