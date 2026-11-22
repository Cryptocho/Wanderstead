//! Original, lit pixel figures. State stays in `game`; these entities are presentation only.
//! Art/layout/provenance: art/figures/{contact-sheet.png,atlas-layout.json,RIGHTS.txt}.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageLoaderSettings, ImageSampler},
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology, view::RenderLayers},
    transform::TransformSystem,
};

use crate::game::*;

pub struct FiguresPlugin;

/// The camera tracks this entity's interpolated world-space foot, never its head/card.
#[derive(Component)]
pub struct PlayerVisual;

const SHADOW_LAYER: usize = 1;
const WALK_TIME: f32 = 0.28;
const ACTION_TIME: f32 = 0.72;
const THROW_DELAY: f32 = 0.25;
const THROW_TIME: f32 = 0.66;

impl Plugin for FiguresPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisualBeat>()
            .add_systems(Startup, spawn_figures)
            .add_systems(
                Update,
                (
                    observe_turn,
                    animate_figures,
                    animate_fruit,
                    light_shadow_layer,
                )
                    .chain()
                    .in_set(crate::Frame::Draw),
            )
            .add_systems(
                PostUpdate,
                face_camera
                    .after(crate::lens::LensSet)
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActorKind {
    Traveler,
    Keeper,
    Jelly,
}

#[derive(Component)]
struct Actor {
    kind: ActorKind,
    from: Vec3,
    to: Vec3,
    age: f32,
    moving: bool,
    facing: IVec2,
    action: ActionKind,
    swallowing: bool,
    fed: bool,
    card: Entity,
    caster: Entity,
    shadow: Entity,
}

#[derive(Component)]
struct FigureCard;

#[derive(Component)]
struct GroundPlum {
    cell: IVec2,
    age: f32,
    delay: f32,
    falling: bool,
}

#[derive(Component)]
struct FruitFlight {
    from: Vec3,
    to: Vec3,
    age: f32,
    delay: f32,
    duration: f32,
    arc: f32,
    shrink: bool,
}

#[derive(Resource, Default)]
struct VisualBeat {
    initialized: bool,
    day: u32,
    turn: u32,
    jelly: IVec2,
}

struct Sheet {
    frames: Vec<Handle<Mesh>>,
    material: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct FigureArt {
    walk: Sheet,
    actions: Sheet,
    keeper: Sheet,
    jelly: Sheet,
    plum: Sheet,
    contact: Handle<Mesh>,
    contact_material: Handle<StandardMaterial>,
}

/// Pixel (width/2,height-2) is the sole/base pivot. The empty two-pixel gutter
/// stays inside each cell, so nearest sampling cannot bleed adjacent frames.
fn card_mesh(
    frame: usize,
    columns: usize,
    rows: usize,
    pixels: Vec2,
    body_pixels: f32,
    height: f32,
) -> Mesh {
    let unit = height / body_pixels;
    let left = -pixels.x * 0.5 * unit;
    let right = -left;
    let bottom = -2.0 * unit;
    let top = (pixels.y - 2.0) * unit;
    let u0 = (frame % columns) as f32 / columns as f32;
    let u1 = ((frame % columns) + 1) as f32 / columns as f32;
    let v0 = (frame / columns) as f32 / rows as f32;
    let v1 = ((frame / columns) + 1) as f32 / rows as f32;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [left, bottom, 0.0],
            [right, bottom, 0.0],
            [right, top, 0.0],
            [left, top, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn sheet(
    path: &'static str,
    grid: UVec2,
    pixels: Vec2,
    body_pixels: f32,
    height: f32,
    server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Sheet {
    let texture = server.load_with_settings(path, |settings: &mut ImageLoaderSettings| {
        settings.sampler = ImageSampler::nearest();
    });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        double_sided: true,
        perceptual_roughness: 0.96,
        reflectance: 0.08,
        ..default()
    });
    let frames = (0..(grid.x * grid.y) as usize)
        .map(|i| {
            meshes.add(card_mesh(
                i,
                grid.x as usize,
                grid.y as usize,
                pixels,
                body_pixels,
                height,
            ))
        })
        .collect();
    Sheet { frames, material }
}

fn spawn_figures(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<Outing>,
) {
    let human = Vec2::new(96.0, 128.0);
    let art = FigureArt {
        walk: sheet(
            "figures/traveler-walk.png",
            UVec2::new(4, 4),
            human,
            108.0,
            1.52,
            &server,
            &mut meshes,
            &mut materials,
        ),
        actions: sheet(
            "figures/traveler-actions.png",
            UVec2::new(4, 4),
            human,
            108.0,
            1.52,
            &server,
            &mut meshes,
            &mut materials,
        ),
        keeper: sheet(
            "figures/keeper-work.png",
            UVec2::new(4, 2),
            human,
            108.0,
            1.50,
            &server,
            &mut meshes,
            &mut materials,
        ),
        jelly: sheet(
            "figures/jelly.png",
            UVec2::new(4, 2),
            Vec2::splat(96.0),
            78.0,
            0.70,
            &server,
            &mut meshes,
            &mut materials,
        ),
        plum: sheet(
            "figures/plum.png",
            UVec2::ONE,
            Vec2::splat(32.0),
            25.0,
            0.20,
            &server,
            &mut meshes,
            &mut materials,
        ),
        contact: meshes.add(Circle::new(0.5)),
        contact_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(server.load_with_settings(
                "figures/contact-shadow.png",
                |settings: &mut ImageLoaderSettings| settings.sampler = ImageSampler::linear(),
            )),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    };
    for (kind, cell, facing) in [
        (ActorKind::Traveler, state.player, state.facing),
        (ActorKind::Keeper, KEEPER, IVec2::X),
        (ActorKind::Jelly, state.jelly, state.jelly_facing),
    ] {
        let body = match kind {
            ActorKind::Traveler => &art.walk,
            ActorKind::Keeper => &art.keeper,
            ActorKind::Jelly => &art.jelly,
        };
        let index = if kind == ActorKind::Traveler {
            direction_row(facing) * 4 + 1
        } else {
            0
        };
        let foot = ground(cell);
        let root = commands
            .spawn((Transform::from_translation(foot), Visibility::default()))
            .id();
        if kind == ActorKind::Traveler {
            commands.entity(root).insert(PlayerVisual);
        }
        let card = commands
            .spawn((
                FigureCard,
                Mesh3d(body.frames[index].clone()),
                MeshMaterial3d(body.material.clone()),
                Transform::default(),
                NotShadowCaster,
            ))
            .id();
        // The camera-facing artwork never tilts the shadow: this second, hidden
        // alpha-cutout stays upright, shares the animation, and casts in sunlight.
        let caster = commands
            .spawn((
                Mesh3d(body.frames[index].clone()),
                MeshMaterial3d(body.material.clone()),
                Transform::default(),
                RenderLayers::layer(SHADOW_LAYER),
            ))
            .id();
        commands.entity(root).add_children(&[card, caster]);
        let shadow = commands
            .spawn((
                Mesh3d(art.contact.clone()),
                MeshMaterial3d(art.contact_material.clone()),
                Transform::from_xyz(foot.x, 0.012, foot.z)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::new(0.53, 0.30, 1.0)),
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();
        commands.entity(root).insert(Actor {
            kind,
            from: foot,
            to: foot,
            age: ACTION_TIME,
            moving: false,
            facing,
            action: ActionKind::Start,
            swallowing: false,
            fed: state.eaten > 0,
            card,
            caster,
            shadow,
        });
    }
    for &cell in &state.fruit {
        spawn_ground_fruit(&mut commands, &art, cell, 0.0, false);
    }
    commands.insert_resource(art);
}

fn spawn_ground_fruit(
    commands: &mut Commands,
    art: &FigureArt,
    cell: IVec2,
    delay: f32,
    falling: bool,
) {
    commands.spawn((
        GroundPlum {
            cell,
            age: 0.0,
            delay,
            falling,
        },
        FigureCard,
        Mesh3d(art.plum.frames[0].clone()),
        MeshMaterial3d(art.plum.material.clone()),
        Transform::from_translation(ground(cell) + Vec3::Y * 0.02),
        if delay > 0.0 || falling {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        },
    ));
}

fn observe_turn(
    mut commands: Commands,
    state: Res<Outing>,
    art: Res<FigureArt>,
    mut beat: ResMut<VisualBeat>,
    mut actors: Query<(&mut Actor, &mut Transform), Without<GroundPlum>>,
    fruit: Query<(Entity, &GroundPlum, &Transform), Without<Actor>>,
    flights: Query<Entity, With<FruitFlight>>,
) {
    if !beat.initialized {
        beat.initialized = true;
        beat.day = state.day;
        beat.turn = state.turn;
        beat.jelly = state.jelly;
        return;
    }
    if beat.day == state.day && beat.turn == state.turn {
        return;
    }
    let new_day = beat.day != state.day;
    for (mut actor, mut transform) in &mut actors {
        if actor.kind == ActorKind::Keeper {
            continue;
        }
        let (cell, facing) = if actor.kind == ActorKind::Traveler {
            (state.player, state.facing)
        } else {
            (state.jelly, state.jelly_facing)
        };
        let target = ground(cell);
        actor.from = transform.translation;
        actor.to = target;
        actor.moving = !new_day && actor.from.distance_squared(target) > 0.001;
        actor.age = if new_day { ACTION_TIME } else { 0.0 };
        actor.action = state.action;
        actor.facing = facing;
        actor.swallowing = !new_day && state.swallowed;
        actor.fed = state.eaten > 0;
        if new_day {
            transform.translation = target;
            actor.from = target;
        }
    }
    if new_day {
        for entity in &flights {
            commands.entity(entity).despawn();
        }
    }
    for (entity, plum, transform) in &fruit {
        if !new_day && state.fruit.contains(&plum.cell) {
            continue;
        }
        if new_day {
            commands.entity(entity).despawn();
        } else if state.swallowed && plum.cell == beat.jelly {
            // Preserve the very fruit removed by the rules for its mouthward motion.
            commands
                .entity(entity)
                .remove::<GroundPlum>()
                .insert(FruitFlight {
                    from: transform.translation,
                    to: ground(state.jelly) + Vec3::new(-0.15, 0.27, 0.05),
                    age: 0.0,
                    delay: 0.06,
                    duration: 0.30,
                    arc: 0.10,
                    shrink: true,
                });
        } else if state.action == ActionKind::Gather {
            commands
                .entity(entity)
                .remove::<GroundPlum>()
                .insert(FruitFlight {
                    from: transform.translation,
                    to: ground(state.player) + Vec3::Y * 0.82,
                    age: 0.0,
                    delay: 0.18,
                    duration: 0.30,
                    arc: 0.20,
                    shrink: true,
                });
        } else {
            commands.entity(entity).despawn();
        }
    }
    for &cell in &state.fruit {
        if !new_day && fruit.iter().any(|(_, plum, _)| plum.cell == cell) {
            continue;
        }
        let delay =
            if matches!(state.action, ActionKind::Throw { to, .. } if to == cell) && !new_day {
                THROW_DELAY + THROW_TIME
            } else {
                0.0
            };
        spawn_ground_fruit(
            &mut commands,
            &art,
            cell,
            delay,
            state.action == ActionKind::Shake && !new_day,
        );
    }
    if !new_day {
        if let ActionKind::Throw { from, to } = state.action {
            commands.spawn((
                FigureCard,
                FruitFlight {
                    from: ground(from) + Vec3::Y * 1.16,
                    to: ground(to) + Vec3::Y * 0.025,
                    age: 0.0,
                    delay: THROW_DELAY,
                    duration: THROW_TIME,
                    arc: 2.0,
                    shrink: false,
                },
                Mesh3d(art.plum.frames[0].clone()),
                MeshMaterial3d(art.plum.material.clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }
    beat.day = state.day;
    beat.turn = state.turn;
    beat.jelly = state.jelly;
}

fn direction_row(direction: IVec2) -> usize {
    if direction.x < 0 {
        1
    } else if direction.x > 0 {
        3
    } else if direction.y < 0 {
        2
    } else {
        0
    }
}

fn animate_figures(
    time: Res<Time>,
    art: Res<FigureArt>,
    mut actors: Query<(&mut Actor, &mut Transform)>,
    mut cards: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        Without<Actor>,
    >,
) {
    for (mut actor, mut root) in &mut actors {
        actor.age += time.delta_secs();
        let mut mirror = false;
        let (sheet, frame) = match actor.kind {
            ActorKind::Traveler => {
                let t = (actor.age / WALK_TIME).clamp(0.0, 1.0);
                let smooth = t * t * (3.0 - 2.0 * t);
                root.translation = actor.from.lerp(actor.to, smooth);
                if actor.age < ACTION_TIME {
                    let row = match actor.action {
                        ActionKind::Gather => Some(0),
                        ActionKind::Shake => Some(1),
                        ActionKind::Throw { .. } => Some(2),
                        ActionKind::Eat => Some(3),
                        _ => None,
                    };
                    if let Some(row) = row {
                        mirror = match actor.action {
                            ActionKind::Throw { from, to } => to.x > from.x,
                            _ => actor.facing.x > 0,
                        };
                        (
                            &art.actions,
                            row * 4 + ((actor.age / ACTION_TIME * 4.0) as usize).min(3),
                        )
                    } else {
                        let step = if actor.moving && t < 1.0 {
                            ((actor.age / WALK_TIME * 4.0) as usize).min(3)
                        } else {
                            1
                        };
                        (&art.walk, direction_row(actor.facing) * 4 + step)
                    }
                } else {
                    (&art.walk, direction_row(actor.facing) * 4 + 1)
                }
            }
            ActorKind::Keeper => {
                // Real arm/utensil poses, not a bobbing static cutout. Work continues
                // while the player considers a turn, without advancing game time.
                let cycle = (time.elapsed_secs() / 0.19) as usize;
                let row = (cycle / 24) % 2;
                (&art.keeper, row * 4 + cycle % 4)
            }
            ActorKind::Jelly => {
                let t = ((actor.age - 0.10) / 0.34).clamp(0.0, 1.0);
                let smooth = t * t * (3.0 - 2.0 * t);
                root.translation = actor.from.lerp(actor.to, smooth);
                if actor.moving && t > 0.0 && t < 1.0 {
                    root.translation.y += 0.24 * (std::f32::consts::PI * t).sin();
                }
                let frame = if actor.swallowing {
                    4 + ((actor.age / 0.70 * 4.0) as usize).min(3)
                } else if actor.moving && actor.age < 0.48 {
                    if actor.age < 0.10 {
                        1
                    } else if t < 0.75 {
                        2
                    } else {
                        3
                    }
                } else if actor.fed {
                    7
                } else {
                    0
                };
                mirror = if frame >= 4 {
                    actor.facing.x > 0
                } else {
                    actor.facing.x < 0
                };
                (&art.jelly, frame)
            }
        };
        for entity in [actor.card, actor.caster] {
            if let Ok((mut mesh, mut material, mut transform)) = cards.get_mut(entity) {
                if mesh.0 != sheet.frames[frame] {
                    mesh.0 = sheet.frames[frame].clone();
                }
                if material.0 != sheet.material {
                    material.0 = sheet.material.clone();
                }
                transform.scale.x = if mirror { -1.0 } else { 1.0 };
            }
        }
        if let Ok((_, _, mut shadow)) = cards.get_mut(actor.shadow) {
            shadow.translation = Vec3::new(root.translation.x, 0.012, root.translation.z);
            let airborne = (root.translation.y / 0.24).clamp(0.0, 1.0);
            let width = if actor.kind == ActorKind::Jelly {
                0.55
            } else {
                0.53
            };
            shadow.scale = Vec3::new(
                width * (1.0 - 0.25 * airborne),
                0.30 * (1.0 - 0.25 * airborne),
                1.0,
            );
        }
    }
}

fn animate_fruit(
    mut commands: Commands,
    time: Res<Time>,
    mut ground_fruit: Query<
        (&mut GroundPlum, &mut Transform, &mut Visibility),
        Without<FruitFlight>,
    >,
    mut flying: Query<
        (Entity, &mut FruitFlight, &mut Transform, &mut Visibility),
        Without<GroundPlum>,
    >,
) {
    for (mut fruit, mut transform, mut visibility) in &mut ground_fruit {
        fruit.age += time.delta_secs();
        let age = fruit.age - fruit.delay;
        *visibility = if age < 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        let endpoint = ground(fruit.cell) + Vec3::Y * 0.025;
        if fruit.falling && age < 0.70 {
            let t = (age / 0.70).clamp(0.0, 1.0);
            let start = ground(TREE) + Vec3::Y * 2.45;
            transform.translation = start.lerp(endpoint, t);
            transform.translation.y += 0.35 * (std::f32::consts::PI * t).sin();
        } else {
            let bounce_age = if fruit.falling { age - 0.70 } else { age };
            let bounce =
                if (0.0..0.22).contains(&bounce_age) && (fruit.falling || fruit.delay > 0.0) {
                    0.14 * (std::f32::consts::PI * bounce_age / 0.22).sin()
                } else {
                    0.0
                };
            transform.translation = endpoint + Vec3::Y * bounce;
        }
    }
    for (entity, mut flight, mut transform, mut visibility) in &mut flying {
        flight.age += time.delta_secs();
        let age = flight.age - flight.delay;
        if age < 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        let t = (age / flight.duration).clamp(0.0, 1.0);
        *visibility = Visibility::Inherited;
        transform.translation =
            flight.from.lerp(flight.to, t) + Vec3::Y * (flight.arc * 4.0 * t * (1.0 - t));
        transform.scale = Vec3::splat(if flight.shrink { 1.0 - 0.85 * t } else { 1.0 });
        if t >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn face_camera(
    camera: Query<&Transform, (With<Camera3d>, Without<FigureCard>)>,
    mut cards: Query<&mut Transform, With<FigureCard>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    // One screen orientation for the complete image plane. Looking at the camera
    // position would toe in outer figures and deform them during the eye-level dolly.
    for mut card in &mut cards {
        card.rotation = camera.rotation;
    }
}

fn light_shadow_layer(
    mut commands: Commands,
    lights: Query<
        (Entity, Option<&RenderLayers>),
        Or<(Added<DirectionalLight>, Added<PointLight>, Added<SpotLight>)>,
    >,
) {
    for (entity, layers) in &lights {
        commands
            .entity(entity)
            .insert(layers.cloned().unwrap_or_default().with(SHADOW_LAYER));
    }
}
