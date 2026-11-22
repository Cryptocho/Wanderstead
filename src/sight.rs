//! One projected, pose-shaped aperture for scenery, character cards, water and picking.
use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
        renderer::{RenderDevice, RenderQueue},
    },
    transform::TransformSystem,
};

pub struct SightPlugin;
impl Plugin for SightPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<SightWindow>::default())
            .add_systems(PreStartup, install)
            .add_systems(
                PostUpdate,
                project
                    .after(TransformSystem::TransformPropagate)
                    .after(bevy::render::camera::CameraUpdateSystem),
            );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(Render, upload.in_set(RenderSet::PrepareResources));
        }
    }
}

// The actual traveler card, not its cell or a synthetic vertical capsule. Each half-size
// contour has 8 texels of extra margin on every edge; the boot pivot is (48,71).
const CELL: UVec2 = UVec2::new(96, 80);
const BOOT: Vec2 = Vec2::new(48., 71.);
pub(crate) const HEIGHT: f32 = 1.32;
pub(crate) const BODY_PIXELS: f32 = 108.;
#[derive(Component, Default)]
pub(crate) struct TravelPose(pub u32);

#[derive(Resource, Clone, ExtractResource)]
pub(crate) struct SightWindow {
    pub buffer: Buffer,
    pub contour: Handle<Image>,
    // Rect: physical pixel origin, contour texels / screen pixel. Meta: inverse view
    // depth, animated frame, reflected horizontally, lowest cuttable world height.
    rect: Vec4,
    meta: Vec4,
    pub enabled: bool,
}
fn install(mut c: Commands, device: Res<RenderDevice>, assets: Res<AssetServer>) {
    let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Traveler sight window"),
        contents: &[0u8; 32],
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    c.insert_resource(SightWindow {
        buffer,
        contour: assets.load_with_settings(
            "figures/traveler-contour.png",
            |s: &mut ImageLoaderSettings| {
                s.is_srgb = false;
                s.sampler = ImageSampler::nearest();
            },
        ),
        rect: Vec4::ZERO,
        meta: Vec4::ZERO,
        enabled: true,
    });
}
fn project(
    mut sight: ResMut<SightWindow>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    body: Query<(&TravelPose, &GlobalTransform, &InheritedVisibility)>,
) {
    let (Ok((cam, eye)), Ok((frame, card, visible))) = (cameras.single(), body.single()) else {
        return;
    };
    let foot = card.translation();
    let plane = (foot - eye.translation()).dot(*eye.forward());
    if !sight.enabled || !visible.get() || plane <= 0.0 {
        if sight.meta.x != 0. {
            sight.meta.x = 0.;
        }
        return;
    }
    let Some(viewport) = cam.physical_viewport_rect() else {
        return;
    };
    let Some(ndc) = cam.world_to_ndc(eye, foot) else {
        return;
    };
    let screen =
        viewport.min.as_vec2() + (Vec2::new(ndc.x, -ndc.y) + 1.) * 0.5 * viewport.size().as_vec2();
    let scale =
        cam.clip_from_view().y_axis.y * viewport.height() as f32 * (HEIGHT / BODY_PIXELS) / plane;
    let rect = (screen - BOOT * scale)
        .extend(scale.recip())
        .extend(scale.recip());
    let flip = card.affine().matrix3.x_axis.dot((*eye.right()).into()) < 0.;
    let meta = Vec4::new(
        plane.recip(),
        frame.0 as f32,
        flip as u8 as f32,
        foot.y + 0.025,
    );
    if sight.rect != rect || sight.meta != meta {
        sight.rect = rect;
        sight.meta = meta;
    }
}
fn upload(sight: Option<Res<SightWindow>>, q: Res<RenderQueue>) {
    let Some(sight) = sight else {
        return;
    };
    if !sight.is_changed() {
        return;
    }
    let [a, b, c, d] = sight.rect.to_array();
    let [e, f, g, h] = sight.meta.to_array();
    q.write_buffer(
        &sight.buffer,
        0,
        [a, b, c, d, e, f, g, h]
            .map(f32::to_ne_bytes)
            .as_flattened(),
    );
}

impl SightWindow {
    /// The open core also admits the mouse ray. A roof projected over a boot must not
    /// intercept a click on the now-visible floor. The uncut feather stays pickable.
    pub fn near(
        &self,
        physical: Vec2,
        ray: Ray3d,
        camera: &GlobalTransform,
        images: &Assets<Image>,
    ) -> f32 {
        if !self.enabled || self.meta.x <= 0. {
            return 0.;
        }
        let xy = (physical - self.rect.truncate().truncate()) * Vec2::new(self.rect.z, self.rect.w);
        if xy.cmplt(Vec2::ZERO).any() || xy.cmpge(CELL.as_vec2()).any() {
            return 0.;
        }
        let mut p = xy.as_uvec2();
        if self.meta.z > 0.5 {
            p.x = CELL.x - 1 - p.x;
        }
        let frame = self.meta.y as u32;
        p += UVec2::new(frame % 8, frame / 8) * CELL;
        if !images
            .get(&self.contour)
            .and_then(|im| im.pixel_bytes(p.extend(0)))
            .is_some_and(|v| v[0] >= 230)
        {
            return 0.;
        }
        let forward = *camera.forward();
        let denom = ray.direction.dot(forward);
        if denom <= 0.0001 {
            return 0.;
        }
        let at = (self.meta.x.recip() - (ray.origin - camera.translation()).dot(forward)) / denom;
        if ray.direction.y < -0.0001 {
            at.min((self.meta.w - ray.origin.y) / ray.direction.y)
                .max(0.)
        } else {
            at.max(0.)
        }
    }
    pub fn description(&self) -> ([f32; 4], [f32; 4]) {
        (self.rect.to_array(), self.meta.to_array())
    }
}
