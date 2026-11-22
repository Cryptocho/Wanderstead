// Four heights of cosine-weighted sky visibility measured in each authored 3D site.
// Interpolate in physical space. Direct sun, torches and emission remain independent.
@group(2) @binding(105) var shelter:texture_2d<f32>;
@group(2) @binding(106) var shelter_sampler:sampler;
fn visible_sky(wp:vec3<f32>,normal:vec3<f32>)->f32 {
    let p=wp+normal*.15;
    if p.y>6.5 || max(abs(p.x),abs(p.z))>15.85{return 1.;}
    let cover=textureSampleLevel(shelter,shelter_sampler,p.xz/32.+.5,0.);
    if p.y<1.30{return mix(cover.r,cover.g,clamp((p.y-.10)/1.20,0.,1.));}
    if p.y<2.65{return mix(cover.g,cover.b,(p.y-1.30)/1.35);}
    if p.y<4.25{return mix(cover.b,cover.a,(p.y-2.65)/1.60);}
    return mix(cover.a,1.,smoothstep(4.25,6.5,p.y));
}
