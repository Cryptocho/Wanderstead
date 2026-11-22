// Original Wanderstead material extension for Bevy 0.16.1.
// StandardMaterial resolves the actual surface: texture, normal, alpha clip,
// roughness, metal, ambient occlusion, sunlight and its cascaded shadow maps.
// Quantization happens in linear HDR *before* Bevy fog/exposure/tonemapping.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_bindings,
    mesh_view_bindings::globals,
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}
#import "inks/sky_shelter.wgsl"::{site_light,street_radiance}
#import "inks/sight_cutaway.wgsl"::hides_traveler

struct PaletteUniform {
    // enabled, world texels/metre, illumination levels/stop, ordered dither amplitude
    settings: vec4<f32>,
    floor_style: vec4<f32>,
    sunward: vec4<f32>,
    single_tap: u32,
}
@group(2) @binding(100) var<uniform> palette: PaletteUniform;
@group(2) @binding(101) var site_edges:texture_2d<f32>;
@group(2) @binding(107) var ground_contact:texture_2d<f32>;
@group(2) @binding(108) var ground_contact_sampler:sampler;

fn edge_distance(bits:u32, f:vec2<f32>)->f32 {
    var d=2.;
    if (bits & 1u)!=0u {d=min(d,f.x);}
    if (bits & 2u)!=0u {d=min(d,1.-f.x);}
    if (bits & 4u)!=0u {d=min(d,f.y);}
    if (bits & 8u)!=0u {d=min(d,1.-f.y);}
    return d;
}
fn stone_grain(p:vec2<f32>)->f32 {
    let q=fract(vec3<f32>(p.xyx)*vec3<f32>(.1031,.11369,.13787));
    return fract((q.x+q.y)*(q.z+dot(q,q.yzx+19.19)));
}

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
    let enabled = palette.settings.x > 0.5;
    // A material pixel has a size in the world, not in the window. The UV
    // Jacobian converts a displacement on the dominant face into atlas space.
    // This also works for rotated bevels and for atlases of different density.
    let geometric = abs(normalize(vertex.world_normal));
    var chart = vertex.world_position.xz;
    if geometric.z >= geometric.y && geometric.z > geometric.x { chart = vertex.world_position.xy; }
    else if geometric.x > geometric.y { chart = vertex.world_position.zy; }
    let pitch = 1.0 / max(palette.settings.y, 1.0);
    let sx = dpdx(chart);
    let sy = dpdy(chart);
#ifdef VERTEX_UVS_A
    let ux = dpdx(vertex.uv);
    let uy = dpdy(vertex.uv);
    let det = sx.x * sy.y - sx.y * sy.x;
#endif
    if hides_traveler(vertex.position,vertex.world_position.y) {discard;}
    var ink_vertex=vertex;
    let drawn_mask=(pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK;
#ifdef VERTEX_UVS_A
    // All PBR channels (normal, roughness and albedo) address the same
    // world-sized material pixel. At grazing angles blend back to derivatives.
    if enabled && !drawn_mask && abs(det)>0.00000001 {
        let coverage=1.-smoothstep(.68,1.20,max(length(sx),length(sy))/pitch);
        let s=(vec2<f32>(.5)-fract(chart/pitch))*(pitch/det);
        ink_vertex.uv+=coverage*((ux*sy.y-uy*sx.y)*s.x+(uy*sx.x-ux*sy.x)*s.y);
    }
#endif
    var pbr = pbr_input_from_standard_material(ink_vertex, is_front);
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    let site=site_light(vertex.world_position.xyz,normalize(pbr.world_normal));
    pbr.diffuse_occlusion*=site.r;pbr.specular_occlusion*=sqrt(site.r);
    // Actual short-range geometry contact, shared by paving, lawn and the
    // bottom of plaster walls. Open ground stays white; metre joints add no AO.
    if palette.floor_style.w>.5 && vertex.world_position.y>-.08 && vertex.world_position.y<.35 {
        let contact_uv=(vertex.world_position.xz+vec2<f32>(16.))/32.;
        if all(contact_uv>=vec2<f32>(0.)) && all(contact_uv<=vec2<f32>(1.)) {
            let contact=textureSampleLevel(ground_contact,ground_contact_sampler,contact_uv,0.).r;
            let weight=1.-smoothstep(.06,.35,vertex.world_position.y);
            pbr.diffuse_occlusion*=mix(1.,contact,weight);
            pbr.specular_occlusion*=mix(1.,sqrt(contact),weight);
            pbr.material.base_color=vec4<f32>(pbr.material.base_color.rgb*mix(1.,.88+.12*contact,weight),pbr.material.base_color.a);
        }
    }
#ifdef VERTEX_UVS_A
    if enabled && !drawn_mask && abs(det) > 0.00000001 &&
        (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        let projected = max(length(sx), length(sy)) / pitch;
        if projected < 1.1 {
            let coverage=.86*(1.-smoothstep(.60,1.1,projected));
            // Town albedos already took one bilinear footprint at the snapped
            // pixel centre in pbr_input_from_standard_material. Nearest-source
            // art from other sites still needs the two-quadrant reconstruction.
            if palette.single_tap==0u {
                let du=(ux*sy.y-uy*sx.y)*(pitch/det);
                let dv=(uy*sx.x-ux*sy.x)*(pitch/det);
                let shift=vec2<f32>(.5)-fract(chart/pitch);
                let middle=vertex.uv+du*shift.x+dv*shift.y;
                let a=.26*(du+dv);
                let filtered=.5*(
                    textureSampleLevel(pbr_bindings::base_color_texture,pbr_bindings::base_color_sampler,middle+a,0.).rgb+
                    textureSampleLevel(pbr_bindings::base_color_texture,pbr_bindings::base_color_sampler,middle-a,0.).rgb);
                var factor=pbr_bindings::material.base_color.rgb;
#ifdef VERTEX_COLORS
                factor*=vertex.color.rgb;
#endif
                pbr.material.base_color=vec4<f32>(mix(pbr.material.base_color.rgb,filtered*factor,coverage),pbr.material.base_color.a);
            }
            pbr.N = normalize(mix(pbr.N,normalize(pbr.world_normal), .34*coverage));
        }
    }
#endif
    if palette.floor_style.x>.5 && palette.floor_style.x<1.5 && geometric.y>.76 && abs(vertex.world_position.y)<.02 {
        let f=fract(vertex.world_position.xz+vec2<f32>(.5));
        let boundary=min(min(f.x,1.-f.x),min(f.y,1.-f.y));
        let seam=1.-smoothstep(.003,.014,boundary);
        pbr.material.base_color=vec4<f32>(pbr.material.base_color.rgb*(1.-.055*seam),pbr.material.base_color.a);
    }
    var reflected_water=0.;
    if enabled && palette.floor_style.x>1.5 && geometric.y>.76 && abs(vertex.world_position.y)<.16 {
        let s=(floor(vertex.world_position.xz/pitch)+.5)*pitch;
        let tile=vec2<i32>(floor(s+.5));
        let f=fract(s+.5);
        let dims=vec2<i32>(textureDimensions(site_edges));let size=vec2<i32>(dims.x,dims.y/2);
        var addr=tile+size/2;
        if all(addr>=vec2<i32>(0)) && all(addr<size) {
            addr.y+=select(0,size.y,palette.floor_style.x>1.5);
            let banks=vec3<u32>(round(textureLoad(site_edges,addr,0).rgb*255.));
            let granite=stone_grain(vec2<f32>(tile));
            let fleck=stone_grain(floor(s*6.3))*.65+stone_grain(floor(s*19.1))*.35;
            let foot=1.-smoothstep(.01,.57,edge_distance(banks.r,f));
            let wet_bank=min(edge_distance(banks.g,f),select(2.,edge_distance(banks.b,f),palette.floor_style.y>.5));
            let damp=(1.-smoothstep(.055,.97,wet_bank))*(.64+.36*fleck);
            if damp>.12 {
                let t=globals.time;
                let ridge=abs(sin(s.x*9.+s.y*3.+.85*sin(s.y*5.-t*.6))+cos(s.y*8.3-t*.57));
                reflected_water=.17*damp*(1.-smoothstep(.023,.17,ridge));
            }
            let seam=min(min(f.x,1.-f.x),min(f.y,1.-f.y));
            let moss=(1.-smoothstep(.01,.105,seam))*smoothstep(.38,.82,fleck)*(.13+.52*foot);
            let flags=pbr.material.base_color.rgb*mix(vec3<f32>(.86,.91,1.01),vec3<f32>(1.10,1.01,.91),granite);
            let roots=flags*vec3<f32>(.52,.80,.42);
            pbr.material.base_color=vec4<f32>(mix(flags,roots,moss)*(1.-damp*.26),pbr.material.base_color.a);
            pbr.material.perceptual_roughness=mix(pbr.material.perceptual_roughness,.30,damp);
            pbr.diffuse_occlusion *= 1.-.46*foot;
        }
    }
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
        let diffuse=pbr.material.base_color.rgb*(1.-pbr.material.metallic);
        let spill=street_radiance(site,pbr.N.y,diffuse);
        out.color=apply_pbr_lighting(pbr)+vec4<f32>(spill+reflected_water*vec3<f32>(.31,.72,.74),0.);
        // Only authored leaf/petal highlight swatches may gain HDR sun tips.
        // Stone is never emissive. The lit-value gate keeps shaded crowns quiet.
        if palette.floor_style.z>.5 {
            let weights=vec3<f32>(.2126,.7152,.0722);
            let paint=dot(pbr.material.base_color.rgb,weights);
            let facing=smoothstep(.20,.68,dot(normalize(pbr.world_normal),palette.sunward.xyz));
            let sunlit=smoothstep(.42,.90,dot(out.color.rgb,weights));
            let selected=step(.83,stone_grain(floor(vertex.world_position.xz*5.7)));
            let tip=smoothstep(select(.40,.62,palette.floor_style.z>1.5),select(.53,.78,palette.floor_style.z>1.5),paint);
            let gleam=selected*tip*facing*sunlit*.78;
            let warm_tip=palette.sunward.w*select(vec3<f32>(1.58,1.43,1.06),vec3<f32>(1.63,1.27,1.20),palette.floor_style.z>1.5);
            out.color=vec4<f32>(mix(out.color.rgb,max(out.color.rgb,warm_tip),gleam),out.color.a);
        }
        if enabled {
            let weights = vec3<f32>(0.2126, 0.7152, 0.0722);
            let luminance = max(dot(out.color.rgb, weights), 0.00001);
            let albedo = max(dot(pbr.material.base_color.rgb, weights), 0.02);
            let cell = vec2<i32>(floor(chart / pitch));
            let town_flags=palette.floor_style.x==1.;
            let threshold=ordered_threshold(cell)*palette.settings.w*select(1.,.34,town_flags);
            let steps = max(palette.settings.z, 1.0);
            // Log illumination bands preserve HDR range at any physical exposure.
            // Soft blending retains contact penumbrae. Metallic highlights are mostly
            // continuous; the surface's roughness and Fresnel remain Bevy's real PBR.
            let illumination = log2(luminance / albedo);
            let band = floor(illumination * steps + 0.5 + threshold) / steps;
            let amount=select(.84,.46,town_flags)*(1.-.82*pbr.material.metallic);
            let band_luminance = albedo * exp2(mix(illumination, band, amount));
            // Cool only the ink in the darkest recesses; keep colored lamps,
            // metal glints, and the real albedo instead of tinting the whole frame.
            let recess = (1.0 - smoothstep(0.045, 0.24, luminance)) * (1.0 - pbr.material.metallic);
            let ink = mix(vec3<f32>(1.0), vec3<f32>(0.91, 0.985, 1.12), recess);
            out.color = vec4<f32>(out.color.rgb * (band_luminance / luminance) * ink, out.color.a);
        }
    }
    return FragmentOutput(main_pass_post_lighting_processing(pbr, out.color));
}
