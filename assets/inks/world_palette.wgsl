// Original Wanderstead material extension for Bevy 0.16.1.
// StandardMaterial resolves the actual surface: texture, normal, alpha clip,
// roughness, metal, ambient occlusion, sunlight and its cascaded shadow maps.
// Quantization happens in linear HDR *before* Bevy fog/exposure/tonemapping.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_bindings,
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct PaletteUniform {
    // enabled, world texels/metre, illumination levels/stop, ordered dither amplitude
    settings: vec4<f32>,
    focus: vec4<f32>,
    eye: vec4<f32>,
}
@group(2) @binding(100) var<uniform> palette: PaletteUniform;

fn ordered_threshold(cell: vec2<i32>) -> f32 {
    let bayer = array<f32, 16>(
         0.0, 8.0, 2.0,10.0,
        12.0, 4.0,14.0, 6.0,
         3.0,11.0, 1.0, 9.0,
        15.0, 7.0,13.0, 5.0,
    );
    let x = u32(cell.x & 3);
    let y = u32(cell.y & 3);
    return (bayer[y * 4u + x] + 0.5) / 16.0 - 0.5;
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var surface = vertex;
    let enabled = palette.settings.x > 0.5;
    // View-local foreground cutaway is independent of the F7 palette switch.
    // The original geometry still casts real shadows. Ground, people and the
    // breakfast props never enter this branch.
    if palette.focus.w > 0.5 && vertex.world_position.y > palette.focus.y - 0.60 {
        let ray = palette.focus.xyz - palette.eye.xyz;
        let ray_length = max(length(ray), 0.001);
        let direction = ray / ray_length;
        let along = dot(vertex.world_position.xyz - palette.eye.xyz, direction);
        if along > 0.35 && along < ray_length - 0.38 {
            let right = normalize(cross(direction, vec3<f32>(0.0, 1.0, 0.0)));
            let up = normalize(cross(right, direction));
            let relative = vertex.world_position.xyz - (palette.eye.xyz + direction * along);
            let oval = vec2<f32>(dot(relative, right) / 0.49, dot(relative, up) / 0.91);
            let radius = dot(oval, oval);
            let cell = vec2<i32>(floor(vertex.world_position.xy * max(palette.settings.y, 1.0)));
            let edge = 0.88 + ordered_threshold(cell) * 0.18;
            if radius < edge { discard; }
        }
    }
    if enabled {
        // Texel centres are in the original UV domain, independent of the camera.
        // Keep all original material texture bindings, including roughness/normal.
#ifdef VERTEX_UVS_A
        if (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
            let dimensions = vec2<f32>(textureDimensions(pbr_bindings::base_color_texture));
            surface.uv = (floor(surface.uv * dimensions) + vec2<f32>(0.5)) / dimensions;
        }
#endif
    }
    var pbr = pbr_input_from_standard_material(surface, is_front);
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    var out: FragmentOutput;
    if (pbr.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u {
        out.color = pbr.material.base_color;
    } else {
        if enabled {
            // Tangentially project the fixed world-cell centre back onto the surface.
            // This avoids moving the light sample inside a floor/wall at contact edges.
            let density = max(palette.settings.y, 1.0);
            let normal = normalize(pbr.world_normal);
            let centre = (floor(pbr.world_position.xyz * density) + vec3<f32>(0.5)) / density;
            let delta = centre - pbr.world_position.xyz;
            pbr.world_position = vec4<f32>(pbr.world_position.xyz + delta - normal * dot(delta, normal), 1.0);
        }
        out.color = apply_pbr_lighting(pbr);
        if enabled {
            let weights = vec3<f32>(0.2126, 0.7152, 0.0722);
            let luminance = max(dot(out.color.rgb, weights), 0.00001);
            let albedo = max(dot(pbr.material.base_color.rgb, weights), 0.02);
            let normal = abs(pbr.world_normal);
            var plane = vertex.world_position.xz;
            if normal.z > normal.y && normal.z >= normal.x {
                plane = vertex.world_position.xy;
            } else if normal.x > normal.y && normal.x > normal.z {
                plane = vertex.world_position.yz;
            }
            let cell = vec2<i32>(floor(plane * max(palette.settings.y, 1.0)));
            let threshold = ordered_threshold(cell) * palette.settings.w;
            let steps = max(palette.settings.z, 1.0);
            // Log illumination bands preserve HDR range at any physical exposure.
            // Soft blending retains contact penumbrae. Metallic highlights are mostly
            // continuous; the surface's roughness and Fresnel remain Bevy's real PBR.
            let illumination = log2(luminance / albedo);
            let band = floor(illumination * steps + 0.5 + threshold) / steps;
            let amount = 0.68 * (1.0 - 0.8 * pbr.material.metallic);
            let band_luminance = albedo * exp2(mix(illumination, band, amount));
            out.color = vec4<f32>(out.color.rgb * (band_luminance / luminance), out.color.a);
        }
    }
    return FragmentOutput(main_pass_post_lighting_processing(pbr, out.color));
}
