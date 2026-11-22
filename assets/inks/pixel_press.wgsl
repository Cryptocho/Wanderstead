#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@group(0) @binding(0) var film:texture_2d<f32>;
const LUMA=vec3<f32>(.2126,.7152,.0722);
@fragment fn fragment(v:FullscreenVertexOutput)->@location(0) vec4<f32>{
    let dims=vec2<i32>(textureDimensions(film));let p=vec2<i32>(v.position.xy);
    // Three screen pixels per art pixel at 1440p. The world is still rendered in full;
    // retain some of its original high-frequency coverage at a high-contrast edge.
    let pitch=max(1,(dims.y+240)/480);
    let c=clamp((p/pitch)*pitch+vec2<i32>(pitch/2),vec2<i32>(0),dims-1);
    let mid=textureLoad(film,c,0).rgb; let raw=textureLoad(film,p,0).rgb;
    let west=textureLoad(film,max(c-vec2<i32>(pitch,0),vec2<i32>(0)),0).rgb;
    let north=textureLoad(film,max(c-vec2<i32>(0,pitch),vec2<i32>(0)),0).rgb;
    let l=dot(mid,LUMA);let contrast=max(dot(west,LUMA),dot(north,LUMA))-l;
    let ink=smoothstep(.035,.22,contrast)*.19*(1.-smoothstep(.45,.87,l));
    let body=mix(mid,raw,.07+.18*smoothstep(.045,.31,abs(dot(raw-mid,LUMA))));
    let dyed=max(body*mix(vec3<f32>(1.),vec3<f32>(.38,.53,.67),ink),vec3<f32>(0.));
    // Quantize in display-like space with a subtle stable four-cell threshold. Bright
    // metal and translucent lamp glints retain their HDR headroom; UI draws afterward.
    let cell=c/pitch;let b=(cell.x&1)+2*(cell.y&1);let threshold=array<f32,4>(.35,.54,.62,.46);
    let display=sqrt(dyed);let blocks=floor(display*36.+threshold[u32(b)])/36.;
    let col=mix(display,blocks,.65);
    return vec4<f32>(col*col,1.);
}
