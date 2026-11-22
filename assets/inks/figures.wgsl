#import bevy_pbr::{forward_io::{VertexOutput, FragmentOutput},pbr_fragment::pbr_input_from_standard_material,pbr_functions::{alpha_discard,apply_pbr_lighting,main_pass_post_lighting_processing}}
#import "inks/sight_cutaway.wgsl"::hides_traveler
#import "inks/sky_shelter.wgsl"::{site_light,street_radiance}
@group(2) @binding(100) var<uniform> allow_cut:u32;
@fragment fn fragment(v:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput {
    if allow_cut!=0u && hides_traveler(v.position,v.world_position.y){discard;}
    var pbr=pbr_input_from_standard_material(v,front);
    pbr.material.base_color=alpha_discard(pbr.material,pbr.material.base_color);
    let site=site_light(v.world_position.xyz,vec3<f32>(0.,1.,0.));
    pbr.diffuse_occlusion*=site.r;pbr.specular_occlusion*=sqrt(site.r);
    // A printed face does not swivel its painted light as one tips the lens.
    // Its rounded sky-facing normal keeps visages legible and still takes lamps and shadows.
    pbr.N=normalize(mix(pbr.N,vec3<f32>(-.14,.76,.63),.67));
    // A standing figure sees about half the lower hemisphere, independent of billboard tilt.
    let lit=apply_pbr_lighting(pbr)+vec4<f32>(street_radiance(site,0.,pbr.material.base_color.rgb),0.);
    return FragmentOutput(main_pass_post_lighting_processing(pbr,lit));
}
