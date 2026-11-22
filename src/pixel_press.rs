//! Final world ink after the lens and tone mapper, before the independent full-resolution UI.
use crate::stage::PaletteState;
use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        RenderApp,
        extract_resource::ExtractResourcePlugin,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{binding_types::texture_2d, *},
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};
use std::sync::Mutex;

pub struct PixelPressPlugin;
impl Plugin for PixelPressPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<PaletteState>::default());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_render_graph_node::<ViewNodeRunner<Press>>(Core3d, WorldInk)
                .add_render_graph_edges(
                    Core3d,
                    (
                        Node3d::Tonemapping,
                        WorldInk,
                        Node3d::EndMainPassPostProcessing,
                    ),
                );
        }
    }
    fn finish(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.init_resource::<Plate>();
        }
    }
}
#[derive(RenderLabel, PartialEq, Eq, Hash, Debug, Clone)]
struct WorldInk;
#[derive(Resource)]
struct Plate {
    layout: BindGroupLayout,
    pipeline: CachedRenderPipelineId,
}
impl FromWorld for Plate {
    fn from_world(world: &mut World) -> Self {
        let layout = world.resource::<RenderDevice>().create_bind_group_layout(
            "World ink source",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        );
        let fragment = world.load_asset("inks/pixel_press.wgsl");
        let pipeline =
            world
                .resource::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("World ink at native resolution".into()),
                    layout: vec![layout.clone()],
                    push_constant_ranges: vec![],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: fragment,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: ViewTarget::TEXTURE_FORMAT_HDR,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: default(),
                    depth_stencil: None,
                    multisample: default(),
                    zero_initialize_workgroup_memory: false,
                });
        Self { layout, pipeline }
    }
}
#[derive(Default)]
struct Press {
    groups: Mutex<[Option<(TextureViewId, BindGroup)>; 2]>,
}
impl ViewNode for Press {
    type ViewQuery = &'static ViewTarget;
    fn run(
        &self,
        _: &mut RenderGraphContext,
        ctx: &mut RenderContext,
        target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if !target.is_hdr()
            || !world
                .get_resource::<PaletteState>()
                .is_some_and(|s| s.enabled)
        {
            return Ok(());
        }
        let plate = world.resource::<Plate>();
        let Some(pipe) = world
            .resource::<PipelineCache>()
            .get_render_pipeline(plate.pipeline)
        else {
            return Ok(());
        };
        let image = target.post_process_write();
        let mut cache = self.groups.lock().unwrap();
        // Bevy ping-pongs between these two exact views. Release both on a resize.
        let i = if cache[0].as_ref().is_some_and(|x| x.0 == image.source.id()) {
            0
        } else if cache[1].as_ref().is_some_and(|x| x.0 == image.source.id()) {
            1
        } else {
            let i = if cache[0].is_none() {
                0
            } else if cache[1].is_none() {
                1
            } else {
                *cache = [None, None];
                0
            };
            cache[i] = Some((
                image.source.id(),
                ctx.render_device().create_bind_group(
                    "World ink",
                    &plate.layout,
                    &BindGroupEntries::single(image.source),
                ),
            ));
            i
        };
        let diagnostics = ctx.diagnostic_recorder();
        let mut pass = ctx
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("World ink"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: image.destination,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(default()),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        let span = bevy::render::diagnostic::RecordDiagnostics::pass_span(
            &diagnostics,
            &mut pass,
            "world_ink",
        );
        pass.set_pipeline(pipe);
        pass.set_bind_group(0, &cache[i].as_ref().unwrap().1, &[]);
        pass.draw(0..3, 0..1);
        span.end(&mut pass);
        Ok(())
    }
}
