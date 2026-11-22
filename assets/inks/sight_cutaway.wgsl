// Shared by color passes only: an occluder still casts its own physically correct shadow.
struct SightWindow { rect:vec4<f32>, pose:vec4<f32> }
@group(2) @binding(102) var<storage,read> sight:SightWindow;
@group(2) @binding(103) var contour:texture_2d<f32>;
fn hides_traveler(frag:vec4<f32>,world_y:f32)->bool {
    // In a perspective view frag.w is the reciprocal view depth. The entire actual
    // billboard is at one depth, even when the nose of the lens is level with its feet.
    if sight.pose.x<=0. || frag.w<=sight.pose.x*(1.+0.000003) || world_y<=sight.pose.w {return false;}
    let p=(frag.xy-sight.rect.xy)*sight.rect.zw;
    if any(p<vec2<f32>(0.)) || any(p>=vec2<f32>(96.,80.)){return false;}
    var cell=vec2<u32>(p);
    if sight.pose.z>.5 {cell.x=95u-cell.x;}
    let frame=u32(sight.pose.y);let origin=vec2<u32>(frame%8u,frame/8u)*vec2<u32>(96,80);
    let coverage=textureLoad(contour,vec2<i32>(origin+cell),0).r;
    let i=(cell.x&3u)+((cell.y&3u)<<2u);
    let bayer=array<u32,16>(0u,8u,2u,10u,12u,4u,14u,6u,3u,11u,1u,9u,15u,7u,13u,5u);
    return coverage>(f32(bayer[i])+.5)/16.;
}
