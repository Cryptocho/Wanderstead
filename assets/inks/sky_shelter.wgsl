// Linear field in world XZ. Interior: RGBA are four sky heights. Outdoor:
// four horizontal tiles, one per height; each packs sky + reflected street RGB.
#import bevy_pbr::mesh_view_bindings::{lights,view}
@group(2) @binding(105) var shelter:texture_2d<f32>;
@group(2) @binding(106) var shelter_sampler:sampler;
fn site_light(wp:vec3<f32>, normal:vec3<f32>)->vec4<f32> {
    let p=wp+normal*.15;
    let open=vec4<f32>(1.,0.,0.,0.);
    if p.y>6.5 || max(abs(p.x),abs(p.z))>15.85{return open;}
    let uv=p.xz/32.+.5;
    let dims=textureDimensions(shelter);
    if dims.x==dims.y {
        let v=textureSampleLevel(shelter,shelter_sampler,uv,0.);
        var sky=v.a;
        if p.y<1.30 {sky=mix(v.r,v.g,clamp((p.y-.10)/1.20,0.,1.));}
        else if p.y<2.65 {sky=mix(v.g,v.b,(p.y-1.30)/1.35);}
        else if p.y<4.25 {sky=mix(v.b,v.a,(p.y-2.65)/1.60);}
        return vec4<f32>(mix(sky,1.,smoothstep(4.25,6.5,p.y)),0.,0.,0.);
    }
    var layer=0.;var t=clamp((p.y-.10)/1.20,0.,1.);
    if p.y>=2.65 {layer=2.;t=clamp((p.y-2.65)/1.60,0.,1.);}
    else if p.y>=1.30 {layer=1.;t=(p.y-1.30)/1.35;}
    let addr=vec2<f32>((uv.x+layer)*.25,uv.y);
    let lo=textureSampleLevel(shelter,shelter_sampler,addr,0.);
    let hi=textureSampleLevel(shelter,shelter_sampler,addr+vec2<f32>(.25,0.),0.);
    return mix(mix(lo,hi,t),open,smoothstep(4.25,6.5,p.y));
}
fn visible_sky(wp:vec3<f32>,normal:vec3<f32>)->f32 {return site_light(wp,normal).r;}

// The bake contains only visibility, reflecting albedo and first incidence.
// Live lux, exposure, receiver albedo and orientation still govern brightness.
fn street_radiance(field:vec4<f32>,normal_y:f32,diffuse:vec3<f32>)->vec3<f32> {
    if lights.n_directional_lights==0u {return vec3<f32>(0.);}
    let form=.5*(1.-clamp(normal_y,-1.,1.));
    return diffuse*field.gba*lights.directional_lights[0].color.rgb*(view.exposure*form/3.141593);
}
