// Pixel-anchored moving normals. The actual town sun / nearby braziers produce
// the highlights; bright bands at inlets are air entrained in moving water.
#import bevy_pbr::{forward_io::{VertexOutput,FragmentOutput}, mesh_view_bindings::globals, pbr_fragment::pbr_input_from_standard_material, pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing}}
struct WaterData { mode:vec4<f32>, lamps:array<vec4<f32>,9>, }
@group(2) @binding(100) var<uniform> data:WaterData;
fn ripple(p:vec2<f32>,t:f32)->vec2<f32> {
    // Rounded bands let ripples travel through the square texels.
    let a=sin(p.x*9.0 + p.y*7.8 + t*2.1 + 0.8*sin(p.y*3.0-t));
    let b=sin(p.x*-11.4 + p.y*14.3-t*2.5+0.7*cos(p.x*2.1+t*.34));
    let c=cos(p.y*23.0 + t*2.2 + sin(p.x*7.0));
    return vec2<f32>(a*.52+b*.35,b*.3+c*.42);
}
@fragment fn fragment(v:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput {
    var pbr=pbr_input_from_standard_material(v,front);
    let t=globals.time;
    let wp=v.world_position.xyz;
    let pos=(floor(wp.xz*38.0)+vec2<f32>(.5))/38.0;
    let slope=ripple(pos,t);
    var ring=0.0;
    if data.mode.x>.5 && data.mode.x<1.5 {
        let d=abs(pos-vec2<f32>(4.0,2.0));
        let r=length(vec2<f32>(max(d.x,d.y)-.85,min(d.x,d.y)));
        ring=(1.0-smoothstep(.03,.7,r))*smoothstep(.76,.96,cos(r*45.0-t*6.1));
    }
    let fresnel=pow(1.0-max(0.0,pbr.V.y),2.8);
    pbr.N=normalize(pbr.N+vec3<f32>(slope.x*.24,0.0,slope.y*.20));
    pbr.material.perceptual_roughness=mix(.26,.15,fresnel);
    let deep=vec3<f32>(.013,.102,.138);
    let shoal=vec3<f32>(.032,.245,.228);
    pbr.material.base_color=vec4<f32>(mix(deep,shoal,clamp(.42+slope.x*.33+fresnel*.2,0.0,1.0)),1.0);
    var col=apply_pbr_lighting(pbr);
    // Opposite-bank reflections break into horizontal, world-stable slivers.
    let wave=smoothstep(.64,.79,slope.y)*smoothstep(.16,.68,slope.x);
    col=vec4<f32>(col.rgb+mix(vec3<f32>(.009,.062,.098),vec3<f32>(.09,.26,.30),fresnel)+vec3<f32>(.12,.24,.25)*wave+vec3<f32>(.24,.48,.42)*ring,1.0);
    if data.mode.x<.5 {
        // Project each authored light across the water plane. Ripples break
        // its reflection into travelling flecks rather than a painted stripe.
        let crest=smoothstep(.53,.93,sin(pos.y*32.+slope.x*3.+t*1.7));
        for(var i=0u;i<9u;i=i+1u) {
            let lamp=data.lamps[i];
            let axis=lamp.xz+pbr.V.xz/max(.18,pbr.V.y)*(lamp.y-wp.y);
            let q=(pos-axis+vec2<f32>(slope.x*.16,slope.y*.13))/vec2<f32>(.75,1.55);
            let mask=max(0.,1.-dot(q,q));
            let hue=select(vec3<f32>(.19,.57,.79),vec3<f32>(1.4,.66,.19),lamp.w>0.);
            col=vec4<f32>(col.rgb+abs(lamp.w)*hue*mask*mask*(.10+crest*.68),1.);
        }
    }
    if data.mode.x>1.5 {
        let drop=fract(v.uv.y*9.0-t*2.6+v.uv.x*.75);
        let strand=smoothstep(.60,.93,drop);
        col=vec4<f32>(col.rgb+vec3<f32>(.19,.39,.42)+strand*vec3<f32>(.32,.47,.43),1.0);
    }
    return FragmentOutput(main_pass_post_lighting_processing(pbr,col));
}
