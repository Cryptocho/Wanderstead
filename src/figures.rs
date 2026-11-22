//! Original, lit pixel figures. State stays in `game`; these entities are presentation only.
//! Art/layout/provenance: art/figures/{contact-sheet.png,atlas-layout.json,RIGHTS.txt}.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageLoaderSettings, ImageSampler},
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{
        mesh::{Indices, MeshAabb},
        primitives::Aabb,
        render_resource::PrimitiveTopology,
        view::{RenderLayers, VisibilitySystems},
    },
    transform::TransformSystem,
};

use crate::game::*;
use crate::{
    delve::{Enemy, EnemyKind},
    map::Area,
};

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
                    animate_feedback,
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
        if std::env::args_os().any(|arg| arg == "--qa") {
            app.add_systems(
                PostUpdate,
                diagnose_figures.after(VisibilitySystems::CheckVisibility),
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActorKind {
    Traveler,
    Keeper,
    Herbalist,
    Jelly,
    Sentry,
    Moth,
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
    enemy_id: Option<u8>,
    hp: u8,
    warn: Option<IVec2>,
    striking: bool,
    dying: bool,
    hurt: bool,
    down: bool,
    pips: Option<[Entity; 3]>,
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

#[derive(Component)]
struct Feedback {
    age: f32,
    duration: f32,
    velocity: Vec3,
}

#[derive(Resource, Default)]
struct VisualBeat {
    initialized: bool,
    day: u32,
    turn: u32,
    jelly: IVec2,
    area: Option<Area>,
    hp: u8,
}

struct Sheet {
    frames: Vec<Handle<Mesh>>,
    material: Handle<StandardMaterial>,
    bounds: Aabb,
}

#[derive(Resource)]
struct FigureArt {
    walk: Sheet,
    actions: Sheet,
    combat: Sheet,
    sentry: Sheet,
    moth: Sheet,
    keeper: Sheet,
    herbalist: Sheet,
    jelly: Sheet,
    plum: Sheet,
    contact: Handle<Mesh>,
    contact_material: Handle<StandardMaterial>,
    spark: Handle<Mesh>,
    hit_material: Handle<StandardMaterial>,
    heal_material: Handle<StandardMaterial>,
    harvest_material: Handle<StandardMaterial>,
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
        base_color: Color::srgb(0.78, 0.78, 0.78),
        base_color_texture: Some(texture.clone()),
        // A small diffuse floor preserves local colours in a deep alcove.
        // Phosphor wings themselves emit light; cloth still takes real shadows.
        emissive_texture: Some(texture),
        emissive: if path == "figures/phosphor-moth.png" {
            LinearRgba::rgb(0.34, 0.38, 0.48)
        } else {
            LinearRgba::rgb(0.075, 0.068, 0.080)
        },
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        double_sided: true,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    let mut bounds = Aabb::default();
    let frames = (0..(grid.x * grid.y) as usize)
        .map(|i| {
            let mesh = card_mesh(
                i,
                grid.x as usize,
                grid.y as usize,
                pixels,
                body_pixels,
                height,
            );
            if i == 0 {
                bounds = mesh
                    .compute_aabb()
                    .expect("figure quad has position bounds");
            }
            meshes.add(mesh)
        })
        .collect();
    // RENDER_WORLD meshes leave CPU Assets after upload. Late-spawned cards
    // therefore need these finite bounds explicitly, including their casters.
    Sheet {
        frames,
        material,
        bounds,
    }
}

fn spawn_figures(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<Outing>,
) {
    let human = Vec2::new(160.0, 128.0);
    let art = FigureArt {
        walk: sheet(
            "figures/traveler-walk.png",
            UVec2::new(4, 4),
            human,
            108.0,
            1.32,
            &server,
            &mut meshes,
            &mut materials,
        ),
        actions: sheet(
            "figures/traveler-actions.png",
            UVec2::new(4, 4),
            human,
            108.0,
            1.32,
            &server,
            &mut meshes,
            &mut materials,
        ),
        combat: sheet(
            "figures/traveler-combat.png",
            UVec2::new(4, 4),
            human,
            108.0,
            1.32,
            &server,
            &mut meshes,
            &mut materials,
        ),
        sentry: sheet(
            "figures/copper-sentry.png",
            UVec2::new(4, 4),
            Vec2::splat(128.0),
            108.0,
            1.34,
            &server,
            &mut meshes,
            &mut materials,
        ),
        moth: sheet(
            "figures/phosphor-moth.png",
            UVec2::new(4, 4),
            Vec2::splat(128.0),
            94.0,
            0.98,
            &server,
            &mut meshes,
            &mut materials,
        ),
        herbalist: sheet(
            "market/herbalist.png",
            UVec2::new(4, 1),
            Vec2::new(96., 128.),
            108.,
            1.31,
            &server,
            &mut meshes,
            &mut materials,
        ),
        keeper: sheet(
            "figures/keeper-work.png",
            UVec2::new(4, 2),
            human,
            108.0,
            1.30,
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
        spark: meshes.add(Sphere::new(0.055)),
        hit_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.56, 0.24),
            emissive: LinearRgba::new(1.8, 0.42, 0.08, 1.0),
            unlit: true,
            ..default()
        }),
        heal_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.36, 1.0, 0.70),
            emissive: LinearRgba::new(0.14, 0.8, 0.42, 1.0),
            unlit: true,
            ..default()
        }),
        harvest_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.90, 0.69, 0.98),
            emissive: LinearRgba::new(0.55, 0.26, 0.7, 1.0),
            unlit: true,
            ..default()
        }),
    };
    for (kind, cell, facing) in [
        (ActorKind::Traveler, state.player, state.facing),
        (ActorKind::Keeper, KEEPER, IVec2::X),
        (ActorKind::Herbalist, crate::map::APOTHECARY, IVec2::NEG_X),
        (ActorKind::Jelly, state.jelly, state.jelly_facing),
    ] {
        spawn_actor(&mut commands, &art, kind, cell, facing, None, state.area);
    }
    for enemy in &state.expedition.enemies {
        spawn_actor(
            &mut commands,
            &art,
            enemy_kind(enemy),
            enemy.pos,
            state.player - enemy.pos,
            Some(enemy),
            state.area,
        );
    }
    if state.area == Area::Town {
        for &cell in &state.fruit {
            spawn_ground_fruit(&mut commands, &art, cell, 0.0, false);
        }
    }
    commands.insert_resource(art);
}

fn enemy_kind(enemy: &Enemy) -> ActorKind {
    match enemy.kind {
        EnemyKind::Sentry => ActorKind::Sentry,
        EnemyKind::Moth => ActorKind::Moth,
    }
}

fn spawn_actor(
    commands: &mut Commands,
    art: &FigureArt,
    kind: ActorKind,
    cell: IVec2,
    facing: IVec2,
    enemy: Option<&Enemy>,
    area: Area,
) {
    let body = match kind {
        ActorKind::Traveler => &art.walk,
        ActorKind::Keeper => &art.keeper,
        ActorKind::Herbalist => &art.herbalist,
        ActorKind::Jelly => &art.jelly,
        ActorKind::Sentry => &art.sentry,
        ActorKind::Moth => &art.moth,
    };
    let index = if kind == ActorKind::Traveler {
        direction_row(facing) * 4 + 1
    } else {
        0
    };
    let foot = ground(cell);
    let visible = if kind == ActorKind::Traveler || (enemy.is_some() == (area == Area::Cistern)) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let root = commands
        .spawn((Transform::from_translation(foot), visible))
        .id();
    if kind == ActorKind::Traveler {
        commands.entity(root).insert(PlayerVisual);
    }
    let card = commands
        .spawn((
            FigureCard,
            Mesh3d(body.frames[index].clone()),
            MeshMaterial3d(body.material.clone()),
            body.bounds,
            Transform::default(),
            NotShadowCaster,
        ))
        .id();
    // Keep the sun caster upright; only the visible artwork faces the camera.
    let caster = commands
        .spawn((
            Mesh3d(body.frames[index].clone()),
            MeshMaterial3d(body.material.clone()),
            body.bounds,
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
            visible,
            NotShadowCaster,
            NotShadowReceiver,
        ))
        .id();
    let pips = enemy.map(|enemy| {
        let pips: [Entity; 3] = std::array::from_fn(|i| {
            commands
                .spawn((
                    Mesh3d(art.spark.clone()),
                    MeshMaterial3d(art.hit_material.clone()),
                    Transform::from_xyz(
                        (i as f32 - 1.0) * 0.13,
                        if kind == ActorKind::Moth { 1.02 } else { 1.48 },
                        0.0,
                    )
                    .with_scale(Vec3::splat(0.60)),
                    if i < enemy.hp as usize {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    NotShadowCaster,
                    NotShadowReceiver,
                ))
                .id()
        });
        commands.entity(root).add_children(&pips);
        pips
    });
    commands.entity(root).insert(Actor {
        kind,
        from: foot,
        to: foot,
        age: ACTION_TIME,
        moving: false,
        facing,
        action: ActionKind::Start,
        swallowing: false,
        fed: false,
        card,
        caster,
        shadow,
        enemy_id: enemy.map(|e| e.id),
        hp: enemy.map_or(0, |e| e.hp),
        warn: enemy.and_then(|e| e.warn),
        striking: false,
        dying: false,
        hurt: false,
        down: false,
        pips,
    });
}

fn burst(commands: &mut Commands, art: &FigureArt, at: Vec3, material: &Handle<StandardMaterial>) {
    for i in 0..7 {
        let angle = i as f32 * std::f32::consts::TAU / 7.0;
        commands.spawn((
            Mesh3d(art.spark.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at),
            Feedback {
                age: 0.0,
                duration: 0.60,
                velocity: Vec3::new(
                    angle.cos() * 0.65,
                    0.75 + (i % 3) as f32 * 0.12,
                    angle.sin() * 0.42,
                ),
            },
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
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
        art.plum.bounds,
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
    mut actors: Query<(Entity, &mut Actor, &mut Transform, &mut Visibility), Without<GroundPlum>>,
    mut bodies: Query<&mut Visibility, Without<Actor>>,
    fruit: Query<(Entity, &GroundPlum, &Transform), Without<Actor>>,
    flights: Query<Entity, With<FruitFlight>>,
    feedback: Query<Entity, With<Feedback>>,
) {
    if !beat.initialized {
        beat.initialized = true;
        beat.day = state.day;
        beat.turn = state.turn;
        beat.jelly = state.jelly;
        beat.area = Some(state.area);
        beat.hp = state.expedition.hp;
        return;
    }
    if beat.day == state.day && beat.turn == state.turn && beat.area == Some(state.area) {
        return;
    }
    let new_day = beat.day != state.day;
    let reset = new_day || beat.area != Some(state.area);
    for (entity, mut actor, mut transform, mut visibility) in &mut actors {
        if let Some(id) = actor.enemy_id {
            if reset || state.area != Area::Cistern {
                commands.entity(entity).despawn();
                commands.entity(actor.shadow).despawn();
                continue;
            }
            if let Some(enemy) = state
                .expedition
                .enemies
                .iter()
                .find(|e| e.id == id && e.hp > 0)
            {
                let hurt = enemy.hp < actor.hp;
                // A hook cancels the warning before it can strike.
                actor.striking = actor.warn.is_some()
                    && enemy.warn.is_none()
                    && !(hurt && state.action == ActionKind::Skill);
                if let Some(target) = actor.warn.or(enemy.warn) {
                    actor.facing = target - enemy.pos;
                } else if actor.to != ground(enemy.pos) {
                    actor.facing = enemy.pos - IVec2::new(actor.to.x as i32, actor.to.z as i32);
                }
                actor.from = transform.translation;
                actor.from.y = 0.0;
                actor.to = ground(enemy.pos);
                actor.moving = actor.from.distance_squared(actor.to) > 0.001;
                actor.warn = enemy.warn;
                actor.hp = enemy.hp;
                actor.hurt = hurt;
                actor.age = 0.0;
            } else if !actor.dying {
                actor.dying = true;
                actor.warn = None;
                actor.hp = 0;
                actor.age = 0.0;
                actor.moving = false;
                actor.to = ground(IVec2::new(actor.to.x as i32, actor.to.z as i32));
                actor.from = actor.to;
                transform.translation = actor.to;
            }
            if let Some(pips) = actor.pips {
                for (i, pip) in pips.into_iter().enumerate() {
                    if let Ok(mut visible) = bodies.get_mut(pip) {
                        *visible = if i < actor.hp as usize {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        };
                    }
                }
            }
            continue;
        }
        let visible = if actor.kind == ActorKind::Traveler || state.area == Area::Town {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *visibility = visible;
        if let Ok(mut shadow) = bodies.get_mut(actor.shadow) {
            *shadow = visible;
        }
        if matches!(actor.kind, ActorKind::Keeper | ActorKind::Herbalist) {
            continue;
        }
        let traveler = actor.kind == ActorKind::Traveler;
        let (cell, facing) = if traveler {
            (state.player, state.facing)
        } else {
            (state.jelly, state.jelly_facing)
        };
        let target = ground(cell);
        actor.from = if reset { target } else { transform.translation };
        actor.to = target;
        actor.moving = !reset && actor.from.distance_squared(target) > 0.001;
        actor.age = if reset { ACTION_TIME } else { 0.0 };
        actor.action = state.action;
        actor.facing = facing;
        actor.swallowing = !reset && state.swallowed;
        actor.fed = state.eaten > 0;
        actor.hurt = traveler && !reset && state.expedition.hp < beat.hp;
        actor.down = traveler && state.expedition.hp == 0;
        if reset {
            transform.translation = target;
        }
    }
    if state.area == Area::Cistern {
        for enemy in &state.expedition.enemies {
            if reset
                || !actors
                    .iter()
                    .any(|(_, actor, _, _)| actor.enemy_id == Some(enemy.id))
            {
                spawn_actor(
                    &mut commands,
                    &art,
                    enemy_kind(enemy),
                    enemy.pos,
                    state.player - enemy.pos,
                    Some(enemy),
                    state.area,
                );
            }
        }
    }
    if reset {
        for entity in &flights {
            commands.entity(entity).despawn();
        }
        for entity in &feedback {
            commands.entity(entity).despawn();
        }
    } else {
        if let Some(cell) = state.expedition.target_hit {
            burst(
                &mut commands,
                &art,
                ground(cell) + Vec3::Y * 0.70,
                &art.hit_material,
            );
        }
        if state.expedition.hp < beat.hp {
            burst(
                &mut commands,
                &art,
                ground(state.player) + Vec3::Y * 0.74,
                &art.hit_material,
            );
        } else if state.expedition.hp > beat.hp || state.action == ActionKind::Eat {
            burst(
                &mut commands,
                &art,
                ground(state.player) + Vec3::Y * 0.70,
                &art.heal_material,
            );
        }
        if state.action == ActionKind::Gather {
            burst(
                &mut commands,
                &art,
                ground(state.player) + Vec3::Y * 0.65,
                &art.harvest_material,
            );
        }
    }
    for (entity, plum, transform) in &fruit {
        if reset || state.area != Area::Town {
            commands.entity(entity).despawn();
        } else if state.fruit.contains(&plum.cell) {
            continue;
        } else if state.swallowed && plum.cell == beat.jelly {
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
    if state.area == Area::Town {
        for &cell in &state.fruit {
            if !reset && fruit.iter().any(|(_, plum, _)| plum.cell == cell) {
                continue;
            }
            let delay =
                if matches!(state.action, ActionKind::Throw { to, .. } if to == cell) && !reset {
                    THROW_DELAY + THROW_TIME
                } else {
                    0.0
                };
            spawn_ground_fruit(
                &mut commands,
                &art,
                cell,
                delay,
                state.action == ActionKind::Shake && !reset,
            );
        }
        if !reset {
            if let ActionKind::Throw { from, to } = state.action {
                commands.spawn((
                    FigureCard,
                    FruitFlight {
                        from: ground(from) + Vec3::Y * 1.03,
                        to: ground(to) + Vec3::Y * 0.025,
                        age: 0.0,
                        delay: THROW_DELAY,
                        duration: THROW_TIME,
                        arc: 2.0,
                        shrink: false,
                    },
                    Mesh3d(art.plum.frames[0].clone()),
                    MeshMaterial3d(art.plum.material.clone()),
                    art.plum.bounds,
                    Transform::default(),
                    Visibility::Hidden,
                ));
            }
        }
    }
    beat.day = state.day;
    beat.turn = state.turn;
    beat.jelly = state.jelly;
    beat.area = Some(state.area);
    beat.hp = state.expedition.hp;
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
    mut commands: Commands,
    time: Res<Time>,
    art: Res<FigureArt>,
    mut actors: Query<(Entity, &mut Actor, &mut Transform)>,
    mut cards: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        Without<Actor>,
    >,
) {
    for (entity, mut actor, mut root) in &mut actors {
        actor.age += time.delta_secs();
        if actor.dying && actor.age >= 1.05 {
            commands.entity(entity).despawn();
            commands.entity(actor.shadow).despawn();
            continue;
        }
        let mut mirror = false;
        let (sheet, frame) = match actor.kind {
            ActorKind::Traveler => {
                let t = (actor.age / WALK_TIME).clamp(0.0, 1.0);
                let smooth = t * t * (3.0 - 2.0 * t);
                root.translation = actor.from.lerp(actor.to, smooth);
                if actor.down {
                    (&art.combat, 8 + ((actor.age / 0.66 * 4.0) as usize).min(3))
                } else if actor.hurt && (0.32..0.64).contains(&actor.age) {
                    mirror = actor.facing.x > 0;
                    (
                        &art.combat,
                        8 + (((actor.age - 0.32) / 0.16) as usize).min(1),
                    )
                } else if actor.age < ACTION_TIME {
                    let combat_row = match actor.action {
                        ActionKind::Strike | ActionKind::Skill => Some(0),
                        ActionKind::Brace => Some(1),
                        ActionKind::Drink => Some(3),
                        _ => None,
                    };
                    let row = match actor.action {
                        ActionKind::Gather => Some(0),
                        ActionKind::Shake => Some(1),
                        ActionKind::Throw { .. } => Some(2),
                        ActionKind::Eat => Some(3),
                        _ => None,
                    };
                    if let Some(row) = combat_row {
                        mirror = actor.facing.x > 0;
                        (
                            &art.combat,
                            row * 4 + ((actor.age / ACTION_TIME * 4.0) as usize).min(3),
                        )
                    } else if actor.action == ActionKind::Hurt {
                        (&art.combat, 8 + ((actor.age / 0.18) as usize).min(1))
                    } else if let Some(row) = row {
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
            ActorKind::Herbalist => {
                let at = (time.elapsed_secs() / 0.44) as usize % 8;
                (&art.herbalist, [0, 0, 1, 2, 2, 3, 0, 0][at])
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
            ActorKind::Sentry | ActorKind::Moth => {
                let t = (actor.age / WALK_TIME).clamp(0.0, 1.0);
                root.translation = actor.from.lerp(actor.to, t * t * (3.0 - 2.0 * t));
                mirror = actor.facing.x > 0;
                let frame = if actor.dying {
                    12 + ((actor.age / 0.72 * 4.0) as usize).min(3)
                } else if actor.striking && actor.age < 0.64 {
                    8 + ((actor.age / 0.64 * 4.0) as usize).min(3)
                } else if actor.warn.is_some() {
                    4 + ((actor.age / 0.48 * 4.0) as usize).min(3)
                } else {
                    ((time.elapsed_secs() / 0.18) as usize) % 4
                };
                (
                    if actor.kind == ActorKind::Sentry {
                        &art.sentry
                    } else {
                        &art.moth
                    },
                    frame,
                )
            }
        };
        for entity in [actor.card, actor.caster] {
            if let Ok((mut mesh, mut material, mut transform, mut visibility)) =
                cards.get_mut(entity)
            {
                if mesh.0 != sheet.frames[frame] {
                    mesh.0 = sheet.frames[frame].clone();
                }
                if material.0 != sheet.material {
                    material.0 = sheet.material.clone();
                }
                transform.scale.x = if mirror { -1.0 } else { 1.0 };
                transform.translation.y = if actor.kind == ActorKind::Moth && !actor.dying {
                    0.26
                } else {
                    0.0
                };
                *visibility = if actor.hurt
                    && !actor.dying
                    && actor.age < 0.30
                    && (actor.age / 0.055) as usize % 2 == 1
                {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
            }
        }
        if let Ok((_, _, mut shadow, _)) = cards.get_mut(actor.shadow) {
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

fn animate_feedback(
    mut commands: Commands,
    time: Res<Time>,
    mut sparks: Query<(Entity, &mut Feedback, &mut Transform)>,
) {
    for (entity, mut spark, mut transform) in &mut sparks {
        spark.age += time.delta_secs();
        if spark.age >= spark.duration {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += spark.velocity * time.delta_secs();
        spark.velocity.y -= time.delta_secs() * 0.7;
        transform.scale = Vec3::splat((1.0 - spark.age / spark.duration).max(0.0));
    }
}

fn face_camera(
    // Camera rotation is copied below; this system never moves a foot pivot.
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

/// On-demand ECS evidence, after transform/visibility propagation. No state changes.
fn diagnose_figures(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<Outing>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    actors: Query<(
        Entity,
        &Actor,
        &GlobalTransform,
        &Visibility,
        &InheritedVisibility,
    )>,
    cards: Query<(
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
        &GlobalTransform,
        &Visibility,
        &InheritedVisibility,
        &ViewVisibility,
        Option<&Aabb>,
        Option<&RenderLayers>,
    )>,
) {
    if !keys.just_pressed(KeyCode::F10) {
        return;
    }
    info!(
        "FIGURES_DUMP area={:?} turn={} logical_enemies={}",
        state.area,
        state.turn,
        state.expedition.enemies.len()
    );
    for (root, actor, global, visibility, inherited) in &actors {
        info!(
            "FIGURE root={root:?} kind={:?} enemy={:?} hp={} foot={:?} visibility={visibility:?} inherited={}",
            actor.kind,
            actor.enemy_id,
            actor.hp,
            global.translation(),
            inherited.get()
        );
        for (label, entity) in [("card", actor.card), ("caster", actor.caster)] {
            match cards.get(entity) {
                Ok((mesh, material, local, global, visible, inherited, view, aabb, layers)) => {
                    let texture = materials
                        .get(&material.0)
                        .and_then(|m| m.base_color_texture.as_ref())
                        .and_then(|t| images.get(t))
                        .map(|image| (image.width(), image.height()));
                    info!(
                        "FIGURE_PART entity={entity:?} role={label} mesh={:?} cpu_mesh={} vertices={:?} aabb={aabb:?} material={:?} material_loaded={} texture={texture:?} local={local:?} world={:?} determinant={} visibility={visible:?} inherited={} view={} layers={layers:?}",
                        mesh.0.id(),
                        meshes.contains(&mesh.0),
                        meshes.get(&mesh.0).map(Mesh::count_vertices),
                        material.0.id(),
                        materials.contains(&material.0),
                        global.translation(),
                        global.affine().matrix3.determinant(),
                        inherited.get(),
                        view.get()
                    );
                }
                Err(error) => {
                    warn!("FIGURE_PART entity={entity:?} role={label} query_error={error:?}")
                }
            }
        }
    }
}
