#import bevy_pbr::{forward_io::VertexOutput,mesh_view_bindings::globals}
#import "inks/sight_cutaway.wgsl"::hides_traveler
@group(2) @binding(0) var<uniform> settings:vec4<f32>; // kind, phase, strength, variant
fn hash(v:vec2<f32>)->f32 {return fract(sin(dot(v,vec2<f32>(37.37,73.21)))*3789.197);}
@fragment fn fragment(v:VertexOutput)->@location(0) vec4<f32> {
    let uv=(floor(v.uv*vec2<f32>(28.0,42.0))+vec2<f32>(.5))/vec2<f32>(28.0,42.0);
    let t=globals.time+settings.y;
    let k=settings.x;
    if (k<3.5 || k>4.5) && hides_traveler(v.position,v.world_position.y){discard;}
    let h=1.0-uv.y;
    // Outdoor air is confined to authored apertures and individual blossoms.
    // Unlike the indoor effects, its edges are continuous rather than card-sized pixels.
    if k>9.5 {
        let fallen=settings.w>1.5;
        let glint=settings.w>.5 && !fallen;
        let phase=min(1.,1.8*fract(globals.time/(8.5+4.*fract(settings.y*.31))+settings.y*.17));
        // Rectangles have V=0 at their top: a falling petal moves toward V=1.
        let center=select(vec2<f32>(.38+.23*phase+.12*sin(phase*8.+settings.y),.12+.74*phase),vec2<f32>(.5),fallen);
        let d=(v.uv-center)*vec2<f32>(1.,1.25);
        let flutter=select(.75+.25*sin(phase*19.+settings.y),.86,fallen);
        let petal=1.-smoothstep(.040,.066,abs(d.x)*flutter+abs(d.y)*.73);
        let life=select(smoothstep(.02,.13,phase)*(1.-smoothstep(.63,.85,phase)),1.,fallen);
        if glint {
            let blink=pow(max(0.,sin(phase*6.28318+settings.y)),18.);
            let point=exp(-dot(d,d)*2600.);
            let a=point*blink*settings.z*life;
            if a<.012 {discard;}
            return vec4<f32>(vec3<f32>(1.25,1.05,.72),a*.55);
        }
        let a=petal*life*settings.z;
        if a<.02 {discard;}
        return vec4<f32>(mix(vec3<f32>(.73,.33,.39),vec3<f32>(1.,.72,.68),smoothstep(-.04,.04,d.x+d.y)),a);
    }
    if k>8.5 {
        // Small, stationary pockets at distant masonry. No world-covering plane.
        let q=(v.uv-.5)*vec2<f32>(2.,2.25);
        let falloff=pow(max(0.,1.-dot(q,q)),2.);
        let a=falloff*settings.z;
        if a<.001 {discard;}
        return vec4<f32>(vec3<f32>(.50,.61,.74),a);
    }
    if k>7.5 {
        let u=v.uv;
        let width=1.-smoothstep(.08,.49,abs(u.x-.5));
        let ends=smoothstep(0.,.13,u.y)*(1.-smoothstep(.48,1.,u.y));
        // Broad, very slow density variations never become isolated glitter pixels.
        let density=.84+.11*sin(u.y*9.+u.x*4.+t*.13+settings.y);
        let a=settings.z*width*ends*density;
        if a<.001 {discard;}
        return vec4<f32>(vec3<f32>(1.,.87,.65),a);
    }
    if k<.5 {
        let bend=(sin(t*5.7+h*6.0)*.077+sin(t*3.8-h*12.0)*.030)*h;
        let x=abs(uv.x-.5-bend);
        let tips=.72+.09*sin(t*6.1)+.075*sin(t*3.37);
        let w=max(0.0,sin(pow(h/tips,.62)*3.14159))*(.31-.105*h);
        let a=smoothstep(-.018,.015,w-x)*step(h,tips);
        if a<.01 {discard;}
        let core=clamp((w-x)*4.2+(1.-h)*.38,0.,1.);
        let orange=mix(vec3<f32>(2.2,.28,.025),vec3<f32>(3.1,1.1,.11),smoothstep(.28,.52,core));
        return vec4<f32>(mix(orange,vec3<f32>(3.0,2.35,1.3),smoothstep(.62,.90,core))*settings.z,a);
    }
    if k<1.5 {
        var plume=0.;
        for(var i=0;i<5;i=i+1){
            let phase=fract(t*.18+f32(i)/5.);
            let c=vec2<f32>(.45+.20*phase+.065*sin(f32(i)*3.3+t*.66),phase);
            let r=mix(.07,.20,phase);
            let d=length((vec2<f32>(uv.x,h)-c)*vec2<f32>(.65,1.));
            plume=max(plume,(1.-smoothstep(r*.58,r,d))*(1.-phase)*smoothstep(0.,.12,phase));
        }
        if plume<.035 {discard;}
        return vec4<f32>(vec3<f32>(.46,.49,.56),plume*.30);
    }
    if k<2.5 {
        let track=uv*vec2<f32>(5.,8.);let id=floor(track);
        let phase=fract(t*.25+hash(id));
        let dotpos=vec2<f32>(.5+.20*sin(phase*6.28+id.y),1.-phase);
        let m=1.-smoothstep(.035,.11,length(fract(track)-dotpos));
        let a=m*smoothstep(.1,.25,phase)*(1.-smoothstep(.6,1.,phase));
        if a<.02 {discard;}return vec4<f32>(mix(vec3<f32>(1.8,1.22,.41),vec3<f32>(.38,1.86,1.53),settings.w),a*.66);
    }
    if k>6.5 {
        // Low veils stay on the water; their scalloped edges dissolve into air.
        let u=v.uv; let sides=smoothstep(0.,.19,u.x)*(1.-smoothstep(.70,1.,u.x));
        let h0=.56+.09*sin(u.x*7.+t*.26)+.075*cos(u.x*16.-t*.30);
        let bank=1.-smoothstep(.03,.33,abs(u.y-h0));
        let layers=.30+.27*sin(u.x*13.+t*.37)+.24*cos(u.x*6.-t*.29);
        return vec4<f32>(vec3<f32>(.34,.54,.66),sides*bank*max(layers,0.)*.26);
    }
    if k>4.5 {
        let u=v.uv;
        let soft=1.-smoothstep(.10,.50,abs(u.x-.5));
        let ends=smoothstep(.0,.075,u.y)*(1.-smoothstep(.67,1.,u.y));
        let leaves=.48+.32*cos(u.x*17.+.4*sin(t*.4+u.y))+.17*sin(u.x*37.-.3*t);
        let mote=hash(floor((u+vec2<f32>(.006*sin(t*.3),t*.008))*vec2<f32>(34.,45.)));
        let dust=select(0.,.60,mote>.990);
        let tint=mix(vec3<f32>(.34,.57,.76),vec3<f32>(1.,.76,.42),settings.w);
        return vec4<f32>(tint,settings.z*soft*ends*(leaves+dust));
    }
    if k>3.5 {
        // A little brass wick-lantern on the traveler's belt. Pixel geometry
        // keeps the iron handle a visible loop at both zoom endpoints.
        let cell=vec2<i32>(floor(v.uv*vec2<f32>(12.,17.)));
        let x=cell.x;let y=cell.y;
        if y<4 {if !((y==0 && x>=4 && x<=7)||(y>=1 && (x==3||x==8))) {discard;}}
        else if y==4 || y==15 {if x<2 || x>9 {discard;}}
        else if y==5 || y==14 {if x<1 || x>10 {discard;}}
        else if y==16 {if x<4 || x>7 {discard;}}
        else {if x<2 || x>9 {discard;}}
        let glass=y>=6 && y<=13 && x>=3 && x<=8;
        return vec4<f32>(select(vec3<f32>(.36,.18,.069),mix(vec3<f32>(2.4,.77,.15),vec3<f32>(3.2,2.05,.65),1.-abs(f32(x)-5.5)*.27),glass),1.);
    }
    let rib=sin(uv.x*53.+sin(t*.7+uv.y*8.)*.9);let bead=fract(uv.y*5.-t*1.42+rib*.27);
    let edge=1.-smoothstep(.31,.49,abs(uv.x-.5));
    return vec4<f32>(mix(vec3<f32>(.06,.31,.40),vec3<f32>(.55,1.1,1.2),smoothstep(.69,.97,bead)),edge*(.25+.48*smoothstep(.02,.9,rib)));
}
