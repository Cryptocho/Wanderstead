// Pixel-anchored moving normals. The actual town sun / nearby braziers produce
// the highlights; bright bands at inlets are air entrained in moving water.
#import bevy_pbr::{forward_io::{VertexOutput,FragmentOutput}, mesh_view_bindings::globals, pbr_fragment::pbr_input_from_standard_material, pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing}}
#import "inks/sky_shelter.wgsl"::visible_sky
@group(2) @binding(104) var banks:texture_2d<f32>;
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
    let pos=(floor(wp.xz*32.0)+vec2<f32>(.5))/32.0;
    // Town modes: 1 repaired basin, 2 falling rivulets, 3 small vessels.
    // Mode 0 below remains the authored underground channel treatment.
    if data.mode.x>.5 {
        let falling=data.mode.x>1.5 && data.mode.x<2.5;
        let basin=data.mode.x<1.5;
        var slope=ripple(pos*.71,t*.48)*.12;
        var impact=0.;
        var foam=0.;
        if basin {
            var landings=array<vec2<f32>,4>(
                vec2<f32>(4.,2.85),vec2<f32>(4.85,2.),
                vec2<f32>(4.,1.15),vec2<f32>(3.15,2.));
            for(var i=0u;i<4u;i=i+1u) {
                let offset=wp.xz-landings[i];
                let r=length(offset);
                let envelope=exp(-r*3.8);
                let phase=r*36.-t*5.1+f32(i)*.37;
                slope+=offset/max(r,.028)*cos(phase)*envelope*.085;
                // A crest spans about one world pixel, rather than disappearing
                // between the 32/m snapped normal samples. Only lit arcs turn silver.
                let crest=smoothstep(.69,.96,sin(phase));
                let arc=.42+.58*smoothstep(-.35,.45,offset.x*.7+offset.y*.5+.14*sin(t*.41+f32(i)));
                impact=max(impact,crest*envelope*arc);
                foam+=exp(-r*r*680.)*(.64+.18*sin(t*7.+f32(i)*2.1));
            }
        }
        if falling {
            // UV.y follows the actual four parabolic streams, not a vertical card.
            let run=sin(v.uv.y*37.-t*8.+sin(v.uv.x*18.+t)*.7);
            let thread=sin(v.uv.x*31.+v.uv.y*13.-t*3.8);
            pbr.N=normalize(pbr.N+vec3<f32>(thread*.09,run*.045,-thread*.07));
            pbr.material.perceptual_roughness=.11;
            pbr.material.metallic=0.;
            pbr.material.base_color=vec4<f32>(.035,.15,.20,.72);
            var col=apply_pbr_lighting(pbr);
            let rim=pow(1.-abs(dot(pbr.N,pbr.V)),2.);
            let bead=smoothstep(.80,.98,run)*smoothstep(.12,.65,v.uv.y);
            let filament=pow(max(0.,cos(v.uv.x*6.28318+v.uv.y*5.-t*.8)),18.);
            col=vec4<f32>(col.rgb+vec3<f32>(.17,.30,.36)*rim+vec3<f32>(.68,.82,.89)*(bead*.53+filament*(.22+bead*.65)),.70+rim*.16+filament*.08);
            return FragmentOutput(main_pass_post_lighting_processing(pbr,col));
        }
        pbr.N=normalize(vec3<f32>(slope.x,1.,slope.y));
        let fresnel=pow(1.-max(0.,dot(pbr.N,pbr.V)),4.);
        let sky=visible_sky(wp,vec3<f32>(0.,1.,0.));
        pbr.diffuse_occlusion*=sky;
        pbr.specular_occlusion*=sqrt(sky);
        pbr.material.metallic=0.;
        pbr.material.perceptual_roughness=.17;
        // Blue-green body colour filters the warm floor without hiding its relief.
        pbr.material.base_color=vec4<f32>(.018,.145,.205,.76);
        var col=apply_pbr_lighting(pbr);
        let reflection=mix(vec3<f32>(.028,.115,.18),vec3<f32>(.22,.34,.43),fresnel)*sky;
        let broken_sky=smoothstep(.66,.87,sin(pos.y*19.+slope.x*11.+t*.36));
        let sun_half=normalize(pbr.V+data.mode.yzw);
        let toward_sun=smoothstep(.86,.98,dot(pbr.N,sun_half));
        let seed=fract(sin(dot(floor(pos*41.),vec2<f32>(37.37,73.21)))*3789.197);
        let glint=smoothstep(.991,.999,seed)*pow(max(0.,sin(t*1.6+seed*113.+pos.x*4.)),10.)*toward_sun;
        let opacity=select(.72,.76,basin)+fresnel*.13;
        col=vec4<f32>(col.rgb+reflection*(.52+broken_sky*.17)+vec3<f32>(.61,.79,.85)*impact*(.55+sky*.45)+vec3<f32>(.63,.75,.76)*foam+vec3<f32>(3.1,2.42,1.60)*glint*sky,opacity);
        return FragmentOutput(main_pass_post_lighting_processing(pbr,col));
    }
    let sky=visible_sky(wp,vec3<f32>(0.,1.,0.));
    pbr.diffuse_occlusion*=sky;pbr.specular_occlusion*=sqrt(sky);
    let slope=ripple(pos,t);
    let fresnel=pow(1.0-max(0.0,pbr.V.y),2.8);
    pbr.N=normalize(pbr.N+vec3<f32>(slope.x*.24,0.0,slope.y*.20));
    pbr.material.perceptual_roughness=mix(.26,.15,fresnel);
    let deep=vec3<f32>(.013,.102,.138);
    let shoal=vec3<f32>(.032,.245,.228);
    let dims=vec2<i32>(textureDimensions(banks));
    let cell=vec2<i32>(floor(pos+.5))+vec2<i32>(dims.x/2,3*dims.y/4);
    let edge=u32(round(textureLoad(banks,cell,0).a*255.));
    let f=fract(pos+vec2<f32>(.5));
    let dist=min(min(select(2.,f.x,(edge&1u)!=0u),select(2.,1.-f.x,(edge&2u)!=0u)),min(select(2.,f.y,(edge&4u)!=0u),select(2.,1.-f.y,(edge&8u)!=0u)));
    let shallows=1.-smoothstep(.03,.73,dist);
    pbr.material.base_color=vec4<f32>(mix(deep,shoal,clamp(.35+shallows*.44+slope.x*.23+fresnel*.2,0.,1.)),1.);
    var col=apply_pbr_lighting(pbr);
    // Opposite-bank reflections break into horizontal, world-stable slivers.
    let wave=smoothstep(.64,.79,slope.y)*smoothstep(.16,.68,slope.x);
    col=vec4<f32>(col.rgb+mix(vec3<f32>(.009,.062,.098),vec3<f32>(.09,.26,.30),fresnel)+vec3<f32>(.12,.24,.25)*wave,1.0);
    let rims=abs(sin(pos.x*8.6+pos.y*3.+1.3*slope.y+t*.65)+.82*cos(pos.y*8.-.95*slope.x-t*.43));
    col=vec4<f32>(col.rgb+vec3<f32>(.09,.33,.27)*shallows*(1.-smoothstep(.035,.19,rims)),1.);
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
    return FragmentOutput(main_pass_post_lighting_processing(pbr,col));
}
