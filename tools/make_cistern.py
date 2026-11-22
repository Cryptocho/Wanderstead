#!/usr/bin/env python3
"""Original ancient spring sanctuary. Blender 4.5 LTS, north-facing, metric grid.
Run python tools/make_cistern_textures.py then blender-4.5 -b -t 4 -P tools/make_cistern.py.
Textures are original generated artwork; geometry is original authored mesh construction.
Game x,y,z -> Blender x,-z,y; every collision wall reads assets/maps/cistern.grid.
"""
import bpy, math, random, json, os
import numpy as np
from pathlib import Path
from mathutils import Vector
R=random.Random(441792)
ROOT=Path(__file__).resolve().parents[1]; OUT=ROOT/'assets/cistern'; ART=ROOT/'art/cistern'
OUT.mkdir(parents=True,exist_ok=True); ART.mkdir(parents=True,exist_ok=True)
GRID=(ROOT/'assets/maps/cistern.grid').read_text().splitlines()
assert len(GRID)==15 and all(len(r)==21 for r in GRID)
def glyph(x,z): return GRID[z+7][x+10] if -10<=x<=10 and -7<=z<=7 else '#'
def cv(p): return (p[0],-p[2],p[1])
def wallheight(x,z):
    if z>=4:return .48
    if x==-10 or x>=9:return 1.15
    if glyph(x,z+1)!='#':return 2.15
    return .74
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
for d in list(bpy.data.materials):bpy.data.materials.remove(d)
M={}
def material(name,color,rough=.75,metal=0,texture=None,emission=0):
    m=bpy.data.materials.new(name);m.use_nodes=True
    m.node_tree.nodes.clear();p=m.node_tree.nodes.new('ShaderNodeBsdfPrincipled');out=m.node_tree.nodes.new('ShaderNodeOutputMaterial');m.node_tree.links.new(p.outputs['BSDF'],out.inputs['Surface']);p.inputs['Base Color'].default_value=(*color,1)
    p.inputs['Roughness'].default_value=rough;p.inputs['Metallic'].default_value=metal
    if emission:
        p.inputs['Emission Color'].default_value=(*color,1);p.inputs['Emission Strength'].default_value=emission
    if texture:
        for suffix,socket,noncolor in [('color','Base Color',False),('roughness','Roughness',True),('normal','Normal',True)]:
            n=m.node_tree.nodes.new('ShaderNodeTexImage');n.image=bpy.data.images.load(str(OUT/f'{texture}_{suffix}.png'),check_existing=True)
            n.interpolation='Linear'
            if noncolor:n.image.colorspace_settings.name='Non-Color'
            if socket=='Normal':
                normal=m.node_tree.nodes.new('ShaderNodeNormalMap');normal.inputs['Strength'].default_value=.65
                m.node_tree.links.new(n.outputs['Color'],normal.inputs['Color']);m.node_tree.links.new(normal.outputs['Normal'],p.inputs['Normal'])
            else:m.node_tree.links.new(n.outputs['Color'],p.inputs[socket])
    M[name]=m;return m
for name,tex in [('Wet paving','paving'),('Ancient masonry','masonry'),('Spring saint relief','spring_saint'),('Crimson spring standard','banner'),('Ivory mosaic','mosaic'),('Pressure bronze','vent'),('Walnut coffer','walnut'),('Carved acanthus','acanthus')]:
    material(name,(.6,.6,.6),texture=tex,metal=.68 if tex=='vent' else 0)
material('Drowned foundations',(.045,.105,.102),.95)
material('Mortar and deep recesses',(.055,.070,.073),.94)
material('Cut limestone edge',(.38,.40,.37),.81)
material('Ivory worn edge',(.56,.55,.45),.75)
material('Verdigris bronze',(.24,.29,.16),.42,.76)
material('Polished bronze edge',(.48,.29,.09),.35,.8)
material('Rusted iron',(.085,.105,.103),.68,.69)
material('Moss shadow',(.095,.14,.057),.97)
material('Moss sun tips',(.21,.25,.079),.93)
material('Fern sage',(.21,.30,.105),.91)
material('Fern dark',(.10,.21,.12),.95)
material('Terracotta',(.27,.16,.095),.84)
material('Candle wax',(.72,.56,.29),.62)
material('Crystal amethyst',(.31,.16,.51),.23,.12)
material('Crystal bright facets',(.58,.36,.73),.31,.1)
material('Cold mineral deposits',(.20,.37,.34),.64)
material('Lamp honey glass',(.90,.39,.08),.28,0,emission=.8)
material('Black slot',(.018,.028,.028),.95)
class Mesh:
    def __init__(self,name,mat,origin=(0,0,0)):
        self.name=name;self.mat=mat;self.origin=Vector(origin);self.v=[];self.f=[];self.uv=[]
    def face(self,pts,uv=None):
        k=len(self.v);self.v.extend(cv(Vector(p)-self.origin) for p in pts);self.f.append(tuple(range(k,k+len(pts))))
        self.uv.append(uv or ([(0,0),(1,0),(1,1),(0,1)] if len(pts)==4 else [(.5+.5*math.cos(i*math.tau/len(pts)),.5+.5*math.sin(i*math.tau/len(pts))) for i in range(len(pts))]))
    def box(self,c,s,rot=0):
        cx,cy,cz=c;xx,yy,zz=[v*.5 for v in s];p=[]
        for x,y,z in [(-xx,-yy,-zz),(xx,-yy,-zz),(xx,-yy,zz),(-xx,-yy,zz),(-xx,yy,-zz),(xx,yy,-zz),(xx,yy,zz),(-xx,yy,zz)]:
            p.append((cx+x*math.cos(rot)+z*math.sin(rot),cy+y,cz-x*math.sin(rot)+z*math.cos(rot)))
        for ids in [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]:self.face([p[i] for i in reversed(ids)])
    def stone(self,c,s):
        cx,cy,cz=c;xx,yy,zz=[a*.5 for a in s]
        shape=[(-.82,-1),(.79,-1),(1,-.72),(1,.78),(.76,1),(-.8,1),(-1,.72),(-1,-.72)]
        lower=[(cx+x*xx,cy-yy,cz+z*zz) for x,z in shape]
        upper=[(cx+x*xx*.97,cy+yy,cz+z*zz*.97) for x,z in shape]
        top=[(cx+x*xx*.91,cy+yy+.018,cz+z*zz*.91) for x,z in shape]
        for i in range(8):
            j=(i+1)%8;self.face([upper[i],upper[j],lower[j],lower[i]]);self.face([top[i],top[j],upper[j],upper[i]])
        self.face(list(reversed(top)),[((x*.91+1)*.5,(z*.91+1)*.5) for x,z in reversed(shape)])
    def beam(self,a,b,r1,r2=None,n=10):
        a=Vector(a);b=Vector(b);d=(b-a).normalized();u=d.cross(Vector((0,1,0)))
        if u.length<.01:u=d.cross(Vector((1,0,0)))
        u.normalize();v=d.cross(u);r2=r1 if r2 is None else r2
        rings=[[p+(u*math.cos(i*math.tau/n)+v*math.sin(i*math.tau/n))*r for i in range(n)] for p,r in [(a,r1),(b,r2)]]
        for i in range(n):self.face([rings[0][i],rings[0][(i+1)%n],rings[1][(i+1)%n],rings[1][i]])
        self.face(list(reversed(rings[0])));self.face(rings[1])
    def torus(self,c,r,t,axis='y',n=24,m=6):
        cx,cy,cz=c
        def pt(a,b):
            u=(r+t*math.cos(b))*math.cos(a);v=(r+t*math.cos(b))*math.sin(a);w=t*math.sin(b)
            return (cx+u,cy+w,cz+v) if axis=='y' else (cx+u,cy+v,cz+w)
        for i in range(n):
            for j in range(m):self.face([pt(i*math.tau/n,j*math.tau/m),pt((i+1)*math.tau/n,j*math.tau/m),pt((i+1)*math.tau/n,(j+1)*math.tau/m),pt(i*math.tau/n,(j+1)*math.tau/m)])
    def finish(self,parent=None):
        if not self.v:return
        me=bpy.data.meshes.new(self.name);me.from_pydata(self.v,[],self.f);me.materials.append(M[self.mat]);me.update();uv=me.uv_layers.new(name='AuthoredSurfaceUV')
        for poly,coords in zip(me.polygons,self.uv):
            for li,co in zip(poly.loop_indices,coords):uv.data[li].uv=co
        ob=bpy.data.objects.new(self.name,me);bpy.context.collection.objects.link(ob);ob.location=cv(self.origin)
        if parent:ob.parent=parent;ob.location=(0,0,0)
        return ob
B={key:Mesh(key,key) for key in M}
def box(mat,c,s,rot=0):B[mat].box(c,s,rot)
def beam(mat,a,b,r,r2=None,n=10):B[mat].beam(a,b,r,r2,n)
def ring(mat,c,r,t,axis='y',n=24):B[mat].torus(c,r,t,axis,n)
def wall_panel(mat,x,z,y,w,h):
    B[mat].face([(x-w/2,y-h/2,z),(x+w/2,y-h/2,z),(x+w/2,y+h/2,z),(x-w/2,y+h/2,z)])
def floor_slab(x,z):
    # Nearly continuous paving set into mortar. Tiny chamfers retain precise grid seams.
    shape=[(-.475,-.490),(.475,-.490),(.490,-.475),(.490,.475),(.475,.490),(-.475,.490),(-.490,.475),(-.490,-.475)]
    upper=[(x+dx,0,z+dz) for dx,dz in shape]
    lower=[(x+dx*1.012,-.035,z+dz*1.012) for dx,dz in shape]
    box('Mortar and deep recesses',(x,-.105,z),(1.005,.14,1.005))
    for i in range(8):
        j=(i+1)%8;B['Wet paving'].face([upper[i],upper[j],lower[j],lower[i]])
    B['Wet paving'].face(list(reversed(upper)),[((x+dx)/1.75,-(z+dz)/1.75) for dx,dz in reversed(shape)])
# Dark physical foundation is visible below the parent-owned moving water, never at y=0.
for z in range(-7,8):
    for x in range(-10,11):
        g=glyph(x,z)
        if g=='~':
            box('Drowned foundations',(x,-.67,z),(.99,.24,.99))
            if R.random()<.45:box('Cold mineral deposits',(x+R.uniform(-.3,.3),-.52,z+R.uniform(-.3,.3)),(.22,.025,.13))
        elif g!='#':
            floor_slab(x,z)
            if glyph(x-1,z)=='~':box('Cut limestone edge',(x-.43,.025,z),(.12,.12,.99))
            if glyph(x+1,z)=='~':box('Cut limestone edge',(x+.43,.025,z),(.12,.12,.99))
            if glyph(x,z-1)=='~':box('Cut limestone edge',(x,.025,z-.43),(.99,.12,.12))
            if glyph(x,z+1)=='~':box('Cut limestone edge',(x,.025,z+.43),(.99,.12,.12))
        else:
            h=wallheight(x,z)
            box('Mortar and deep recesses',(x,h*.5-.14,z),(.935,h+.28,.935))
            exposed=any(glyph(x+dx,z+dz)!='#' for dx,dz in [(1,0),(-1,0),(0,1),(0,-1)])
            if exposed:
                rows=max(2,int(h/.26));rh=h/rows
                # True masonry courses, projecting alternating blocks on every exposed face.
                for k in range(rows):
                    for dx in (-.25,.25):
                        for dz in (-.25,.25):
                            if ((dx<0 and glyph(x-1,z)!='#') or (dx>0 and glyph(x+1,z)!='#') or (dz<0 and glyph(x,z-1)!='#') or (dz>0 and glyph(x,z+1)!='#')):
                                B['Ancient masonry'].stone((x+dx,k*rh+rh/2-.035,z+dz),(.488,rh-.025,.488))
                B['Ancient masonry'].stone((x,h+.025,z),(.998,.075,.998))
                if z<4:
                    box('Carved acanthus',(x,h-.22,z),(.999,.16,1.0))
            else: B['Ancient masonry'].stone((x,h-.055,z),(.985,.12,.985))
# A single broken inlaid aqueduct line gathers into the east altar, never repeated
# as a one-emblem-per-cell tile. Tesserae are shallow geometry inset into the paving.
def tessera(x,z,angle=0):
    if glyph(round(x),round(z)) not in '#~':
        mat='Ivory worn edge' if R.random()<.42 else 'Cold mineral deposits'
        box(mat,(x,.004,z),(.051,.008,.047),angle)
for side in [-1,1]:
    for j in range(87):
        zz=3.65-j*.083;xx=6+side*.26+math.sin(zz*1.8)*.105
        if j%13 not in (5,6,7):tessera(xx,zz,math.cos(zz*1.8)*.18)
for i in range(100):
    a=i*math.tau/100
    if i%17 not in (7,8,9,10):
        tessera(6+math.cos(a)*1.35,-4+math.sin(a)*1.35,a)
# Physical channels have thick kerbs at the # boundary, not a one-polygon dark pool.
# Decorative fluted columns only occupy already-blocked stone cells.
def column(x,z,height=2.65,scale=1):
    r=.205*scale
    for y,s in [(.10,.56),(.23,.45),(height-.25,.44),(height-.10,.59)]:
        B['Carved acanthus'].stone((x,y,z),(s*scale,.13,s*scale))
    beam('Ivory worn edge',(x,.28,z),(x,height-.34,z),r,r*.86,16)
    for a in range(12):
        ang=a*math.tau/12;dx=math.cos(ang)*r;dz=math.sin(ang)*r
        beam('Ancient masonry',(x+dx,.34,z+dz),(x+dx*.86,height-.36,z+dz*.86),.022,.018,5)
    for y in [.31,.43,height-.40,height-.30]:ring('Ivory worn edge',(x,y,z),r*1.05,.032)
    # Four carved scroll volutes.
    for dx in [-.19,.19]:ring('Carved acanthus',(x+dx,height-.19,z+.10),.075,.032,'z',12)
for x,z in [(-6,-6),(-4,-6),(-1,-5),(1,-5),(4,-7),(7,-7),(-3,-3),(3,-3),(-3,1),(3,1)]:
    assert glyph(x,z)=='#'
    column(x,z+.38,2.65 if z< -4 else 1.45,.90 if z>=-4 else 1)
# Voussoir arched framing, all its support rests within the blocked north facade.
def arch(x,z,bottom,radius,thickness):
    n=15
    for i in range(n):
        a=i*math.pi/n;b=(i+1)*math.pi/n;ro=radius+thickness;ri=radius
        p=[(x+math.cos(t)*r,bottom+math.sin(t)*r,zz) for zz in (z-.12,z+.12) for t,r in [(a,ri),(b,ri),(b,ro),(a,ro)]]
        for ids in [(0,1,2,3),(7,6,5,4),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0)]:B['Carved acanthus'].face([p[j] for j in ids])
    for dx in [-radius-thickness/2,radius+thickness/2]:
        box('Carved acanthus',(x+dx,bottom/2,z),(.18,bottom,.25))
arch(0,-4.47,1.48,.72,.18)
arch(-5,-5.49,1.45,.76,.17)
arch(5.5,-6.49,1.4,.80,.19)
# Relief is a shallow actual sculpted grid, not a billboard background. The generated
# material is UV-mapped into geometry inside a thick recessed architectural niche.
def relief(x,z,y,w,h):
    box('Black slot',(x,y,z-.07),(w-.10,h-.10,.08))
    mesh=B['Spring saint relief'];im=bpy.data.images.load(str(OUT/'spring_saint_color.png'),check_existing=True)
    pix=np.asarray(im.pixels[:]).reshape(im.size[1],im.size[0],4)
    n=40
    def pt(i,j):
        u=i/n;v=j/n;val=pix[min(511,int(v*511)),min(511,int(u*511)),:3].mean()
        return (x+(u-.5)*w,y+(v-.5)*h,z+float(val)*.038)
    for j in range(n):
        for i in range(n):mesh.face([pt(i,j),pt(i+1,j),pt(i+1,j+1),pt(i,j+1)],[(i/n,j/n),((i+1)/n,j/n),((i+1)/n,(j+1)/n),(i/n,(j+1)/n)])
    for dx in [-w/2-.06,w/2+.06]:box('Carved acanthus',(x+dx,y,z+.015),(.12,h+.23,.15))
    for yy in [y-h/2-.06,y+h/2+.06]:box('Ivory worn edge',(x,yy,z+.025),(w+.25,.12,.15))
relief(0,-4.46,1.25,1.22,1.62)
# Western service alcove: a real carved cabinet, hanging bronze implements and vials.
box('Walnut coffer',(-5,1.18,-5.46),(1.32,1.35,.13))
for dx in [-.70,.70]:box('Carved acanthus',(-5+dx,1.18,-5.38),(.12,1.48,.18))
for yy in [.52,1.76]:box('Carved acanthus',(-5,yy,-5.31),(1.53,.12,.32))
beam('Rusted iron',(-5.58,1.59,-5.29),(-4.42,1.59,-5.29),.025,n=8)
for i in range(5):
    xx=-5.46+i*.23;length=.30+(i%3)*.08
    ring('Polished bronze edge',(xx,1.55,-5.255),.042,.012,'z',12)
    beam('Verdigris bronze',(xx,1.50,-5.25),(xx,1.50-length,-5.25),.018,n=6)
    if i%2:box('Polished bronze edge',(xx,1.50-length,-5.24),(.12,.07,.035))
    else:ring('Verdigris bronze',(xx,1.48-length,-5.25),.065,.020,'z',12)
for dx in [-.48,-.24,.08,.34,.49]:
    beam('Terracotta',(-5+dx,.60,-5.25),(-5+dx,.77,-5.25),.046,.055,8)
    beam('Lamp honey glass',(-5+dx,.77,-5.25),(-5+dx,.87,-5.25),.031,n=8)
    box('Candle wax',(-5+dx,.89,-5.25),(.05,.04,.05))
# Lower offering basin, solid and empty; its footprint is the blocked wall cell.
for x,z in [(0,-4.80),(-5,-5.80)]:
    box('Carved acanthus',(x,.15,z),(.68,.27,.40))
    box('Black slot',(x,.302,z),(.49,.012,.24))
# Folded red banners are true 3D strips with their own original embroidery texture.
def banner(x,z,y,w=.57,h=1.30):
    mesh=B['Crimson spring standard'];nu=12;nv=8
    for j in range(nv):
        for i in range(nu):
            pts=[];uv=[]
            for ii,jj in [(i,j),(i+1,j),(i+1,j+1),(i,j+1)]:
                u=ii/nu;v=jj/nv;wave=math.sin(u*math.tau*2.5)*.035
                pts.append((x+(u-.5)*w,y+(v-.5)*h,z+wave));uv.append((u,v))
            mesh.face(pts,uv)
    beam('Polished bronze edge',(x-w*.62,y+h*.53,z),(x+w*.62,y+h*.53,z),.026,n=8)
    for dx in [-w*.57,w*.57]:beam('Verdigris bronze',(x+dx,y+h*.53,z),(x+dx,y+h*.72,z-.05),.018,n=6)
banner(-6,-5.46,1.20,.52,1.24);banner(-4,-5.46,1.20,.52,1.24)
banner(1,-4.45,1.38,.44,1.20);banner(7,-6.46,1.36,.62,1.55)
# Drowned north sluices, dark barrel mouths framed by real corbelled stone.
for x in [-8.5,4.5,6.5]:
    z=-6.49 if x>0 else -6.49
    wall_panel('Black slot',x,z,.74,.74,1.14)
    arch(x,z+.02,.85,.39,.12)
    for dx in [-.24,0,.24]:beam('Rusted iron',(x+dx,.25,z+.045),(x+dx,1.02,z+.045),.022,n=6)
# Copper pipes and leakage: side conduit runs entirely within blocked walls.
for x,zs in [(-9.47,range(-5,5)),(8.47,range(-4,4))]:
    for z in zs:
        beam('Verdigris bronze',(x,.54,z-.48),(x,.54,z+.48),.056,n=8)
        for dz in [-.43,.43]:ring('Polished bronze edge',(x,.54,z+dz),.077,.018,'z',12)
# Authored coffer bodies and independently hinged lids with local X opening axis.
def chest(x,z,key):
    body=Mesh(key+'_walnut','Walnut coffer');trim=Mesh(key+'_bronze','Verdigris bronze');inside=Mesh(key+'_interior','Black slot')
    for dx in [-.36,.36]:body.box((x+dx,.31,z),(.10,.49,.58))
    for dz in [-.25,.25]:body.box((x,.31,z+dz),(.72,.49,.10))
    body.box((x,.10,z),(.80,.09,.58));inside.box((x,.17,z),(.61,.08,.40))
    for dx in [-.31,.31]:
        trim.box((x+dx,.33,z+.306),(.057,.5,.034));trim.box((x+dx,.33,z-.306),(.057,.5,.034))
        for dz in [-.20,.20]:trim.box((x+dx,.065,z+dz),(.13,.12,.15))
    trim.box((x,.39,z+.325),(.12,.15,.045));trim.torus((x,.39,z+.36),.04,.012,'z',12)
    for b in [body,trim,inside]:b.finish()
    hinge=bpy.data.objects.new(key+'_lid',None);bpy.context.collection.objects.link(hinge);hinge.location=cv((x,.55,z-.29))
    lid=Mesh(key+'_lid_wood','Walnut coffer',(x,.55,z-.29));ed=Mesh(key+'_lid_bands','Polished bronze edge',(x,.55,z-.29))
    lid.box((x,.62,z),(.82,.16,.64))
    # Arched strips create a low barrel-vaulted top.
    for i in range(8):
        a=-math.pi/2+i*math.pi/8;b=-math.pi/2+(i+1)*math.pi/8
        za=z+math.sin(a)*.32;zb=z+math.sin(b)*.32
        ya=.64+math.cos(a)*.13;yb=.64+math.cos(b)*.13
        lid.face([(x-.40,ya,za),(x+.40,ya,za),(x+.40,yb,zb),(x-.40,yb,zb)])
        for dx in [-.29,.29]:ed.face([(x+dx-.027,ya+.007,za),(x+dx+.027,ya+.007,za),(x+dx+.027,yb+.007,zb),(x+dx-.027,yb+.007,zb)])
    lid.finish(hinge);ed.finish(hinge)
    # Loot is recessed and remains visible only after a real lid rotation.
    if key=='tool':
        beam('Polished bronze edge',(x-.19,.26,z),(x+.19,.26,z),.04,n=8)
        ring('Verdigris bronze',(x,.30,z),.15,.025,'y',16)
    else:
        for dx in [-.18,.05,.22]:
            beam('Lamp honey glass',(x+dx,.20,z),(x+dx,.39,z),.045,n=8)
            box('Candle wax',(x+dx,.41,z),(.065,.05,.065))
chest(-6,0,'tool');chest(-5,-4,'tonic')
# East reliquary: stepped ivory altar and removable actual bronze handwheel.
for y,size in [(.06,.87),(.18,.75),(.33,.61),(.49,.81)]:
    B['Carved acanthus'].stone((6,y,-4),(size,.12,size))
box('Black slot',(6,.558,-4),(.62,.015,.60))
wheelroot=bpy.data.objects.new('wheel_pickup',None);bpy.context.collection.objects.link(wheelroot);wheelroot.location=cv((6,.86,-4))
wh=Mesh('wheel_pickup_bronze','Polished bronze edge',(6,.86,-4));wh.torus((6,.86,-4),.30,.038,'z',32)
for i in range(6):
    a=i*math.tau/6;wh.beam((6,.86,-4),(6+math.cos(a)*.285,.86+math.sin(a)*.285,-4),.024,n=8)
wh.beam((6,.86,-4.09),(6,.86,-3.91),.065,n=12);wh.finish(wheelroot)
# Crystal clumps stay on the blocked east pillars, not on the movement tiles.
def crystal(x,z,y,size):
    for i in range(5):
        xx=x+R.uniform(-.18,.18)*size;zz=z+R.uniform(-.18,.18)*size;h=R.uniform(.28,.73)*size;r=R.uniform(.055,.105)*size
        n=5;lo=[];hi=[]
        for j in range(n):
            a=j*math.tau/n;lo.append((xx+math.cos(a)*r,y,zz+math.sin(a)*r));hi.append((xx+math.cos(a)*r*.86,y+h*.72,zz+math.sin(a)*r*.86))
        for j in range(n):
            mat='Crystal bright facets' if j%2 else 'Crystal amethyst';B[mat].face([lo[j],lo[(j+1)%n],hi[(j+1)%n],hi[j]])
            B[mat].face([hi[j],hi[(j+1)%n],(xx+.02,y+h,zz)])
for p in [(9,0,1.2,1.1),(3,-3,1.46,.85),(7,-7,2.73,.7)]:crystal(*p)
# Pressure lids: a truly flat walkable slotted cover with sculpted rim and screws.
for z in range(-7,8):
    for x in range(-10,11):
        if glyph(x,z)=='P':
            box('Black slot',(x,-.005,z),(.85,.012,.85))
            ring('Polished bronze edge',(x,.020,z),.40,.026,n=32)
            # Generated full-face vent relief plus thin physical iron grille bars.
            B['Pressure bronze'].face([(x-.36,.012,z+.36),(x+.36,.012,z+.36),(x+.36,.012,z-.36),(x-.36,.012,z-.36)])
            for dz in [-.24,-.12,0,.12,.24]:box('Rusted iron',(x,.030,z+dz),(.56,.018,.026))
            for a in range(8):
                ang=a*math.tau/8;beam('Polished bronze edge',(x+math.cos(ang)*.385,.02,z+math.sin(ang)*.385),(x+math.cos(ang)*.385,.045,z+math.sin(ang)*.385),.025,n=6)
# Return stair is walkable, shallow treads descend from the exact < tile into the path.
for i in range(6):
    z=4.65+i*.14;h=.018+i*.012
    B['Cut limestone edge'].stone((0,h*.5,z),(.91,h,.135))
for dx in [-.43,.43]:box('Polished bronze edge',(dx,.019,5),(.022,.018,.91))
# Eroded terracotta vessels, rubble and ferns grow only within # cells or floor edges.
def pot(x,z,y=.0,s=1):
    beam('Terracotta',(x,y+.06*s,z),(x,y+.32*s,z),.11*s,.17*s,10)
    beam('Terracotta',(x,y+.32*s,z),(x,y+.45*s,z),.17*s,.10*s,10)
    ring('Terracotta',(x,y+.45*s,z),.11*s,.025*s,n=12)
    box('Black slot',(x,y+.445*s,z),(.14*s,.005,.14*s))
for x,z in [(-3,0),(3,0),(-7,5),(5,5),(-4,-6)]:
    if glyph(x,z)=='#':pot(x+.19,z+.15,wallheight(x,z)+.08,.65)
def fern(x,z,y,s):
    for a in range(5):
        ang=a*math.tau/5+R.random()*.3;dx=math.cos(ang);dz=math.sin(ang)
        for j in range(1,5):
            t=j/5;cx=x+dx*t*.26*s;cz=z+dz*t*.26*s;cy=y+math.sin(t*2.1)*.30*s
            length=(1-t)*.14*s
            for side in [-1,1]:
                ex=cx+dz*side*length;ez=cz-dx*side*length
                B['Fern sage' if a%2 else 'Fern dark'].face([(cx,cy,cz),(ex,cy+.018*s,ez),(cx+dx*.07*s,cy+.045*s,cz+dz*.07*s)])
# Preserve cell centres: low growth and grit is limited to the outermost .13m.
for z in range(-6,7):
    for x in range(-9,9):
        g=glyph(x,z)
        if g=='#' and any(glyph(x+dx,z+dz)!='#' for dx,dz in [(1,0),(-1,0),(0,1),(0,-1)]):
            h=wallheight(x,z)
            for i in range(7):
                xx=x+R.uniform(-.43,.43);zz=z+R.uniform(-.43,.43)
                B['Moss shadow' if i%2 else 'Moss sun tips'].stone((xx,h+.073,zz),(R.uniform(.045,.14),.009,R.uniform(.045,.12)))
            if R.random()<.66:fern(x+R.uniform(-.25,.25),z+.2,h+.09,R.uniform(.7,1.3))
        elif g not in '#~' and R.random()<.55:
            nearwall=any(glyph(x+dx,z+dz) in '#~' for dx,dz in [(1,0),(-1,0),(0,1),(0,-1)])
            if nearwall:
                side=R.choice([-1,1]);xx=x+side*.40;zz=z+R.uniform(-.3,.3)
                for i in range(2):B['Moss shadow'].stone((xx+R.uniform(-.03,.03),.013,zz+R.uniform(-.2,.2)),(.12,.015,.13))
                if R.random()<.30:fern(xx,zz,.02,.45)
                if R.random()<.40:B['Ancient masonry'].stone((xx,.032,zz),(.10,.04,.08))
# Botanical silhouettes drape down real wall faces; each ivy leaf has folded 3D lobes.
def ivy(x,z,height):
    for strand in range(5):
        xx=x+(strand-2)*.13;zz=z+.514;length=R.uniform(.42,.93);last=(xx,height+.08,zz)
        for j in range(9):
            t=(j+1)/9;cx=xx+math.sin(t*5+strand)*.09;cy=height+.08-length*t;cz=zz+math.sin(t*3)*.025
            beam('Fern dark',last,(cx,cy,cz),.006,n=4);last=(cx,cy,cz)
            for side in [-1,1]:
                sz=R.uniform(.043,.071);tip=(cx+side*sz*1.4,cy+.025,cz+.018)
                base=(cx,cy-.025,cz);a=(cx+side*sz*.6,cy+sz*.75,cz+.015);b=(cx+side*sz*.8,cy-sz*.55,cz+.012);mid=(cx+side*sz*.72,cy+.007,cz+.045)
                mat='Fern sage' if (j+strand)%3 else 'Moss sun tips'
                B[mat].face([base,a,mid]);B[mat].face([a,tip,mid]);B[mat].face([tip,b,mid]);B[mat].face([b,base,mid])
for x,z in [(-7,-6),(-3,-4),(-3,-1),(3,-4),(3,0),(8,-7),(9,1),(-7,5),(1,-5),(-4,-6)]:
    assert glyph(x,z)=='#'
    ivy(x,z,wallheight(x,z))
# Lamps are real, legible metal bowls/lanterns; emissive glass only, parent owns flames/lights.
LAMPS=[
 {'name':'western_offering','position':[-6.25,1.24,-5.32],'color':[1.0,.52,.20],'range':4.8,'lumens':2700,'zone':'western moss archive'},
 {'name':'tool_warm','position':[-3.12,1.08,.02],'color':[1.0,.49,.16],'range':4.3,'lumens':2300,'zone':'western coffer'},
 {'name':'saint_gold','position':[-.95,1.24,-4.32],'color':[1.0,.58,.26],'range':4.8,'lumens':3100,'zone':'central spring shrine'},
 {'name':'nave_gold','position':[3.12,1.09,.90],'color':[1.0,.53,.20],'range':4.5,'lumens':2400,'zone':'central nave and pressure covers'},
 {'name':'return_lantern','position':[-2.96,1.00,5.00],'color':[1.0,.55,.24],'range':4.8,'lumens':2600,'zone':'return stair approach'},
 {'name':'east_lantern','position':[9.10,1.56,-3.10],'color':[1.0,.62,.30],'range':4.5,'lumens':2400,'zone':'eastern wheel altar'},
 {'name':'east_amethyst','position':[8.82,1.55,.08],'color':[.56,.34,1.0],'range':3.3,'lumens':1050,'zone':'east crystal annex','type':'crystal'},
 {'name':'north_seep','position':[5.5,.48,-5.9],'color':[.20,.65,.76],'range':5.0,'lumens':1700,'zone':'north flooded channel','type':'seep'},
 {'name':'west_seep','position':[-8.5,.36,-.80],'color':[.17,.62,.71],'range':5.2,'lumens':1450,'zone':'western channel','type':'seep'},
]
for l in LAMPS:
    if l.get('type'):continue
    x,y,z=l['position']
    beam('Verdigris bronze',(x,y-.43,z),(x,y-.16,z),.085,.115,10)
    beam('Polished bronze edge',(x,y-.15,z),(x,y-.06,z),.19,.22,12)
    ring('Polished bronze edge',(x,y-.055,z),.22,.027,n=16)
    beam('Black slot',(x,y-.045,z),(x,y-.035,z),.16,n=12)
    for dx,dz in [(-.09,0),(.07,.055),(0,-.07)]:
        beam('Candle wax',(x+dx,y-.035,z+dz),(x+dx,y+.075,z+dz),.029,n=6)
        beam('Lamp honey glass',(x+dx,y+.07,z+dz),(x+dx,y+.105,z+dz),.022,.009,6)
# A single batch per shared material, with only 3 independent state roots.
for batch in B.values():batch.finish()
scene=bpy.context.scene
scene['source']='Original Wanderstead cistern; assets/maps/cistern.grid authoritative'
scene['water_y']=-.22
# Export does not include preview camera or lights. scene0 is the complete playable geometry.
bpy.ops.export_scene.gltf(filepath=str(OUT/'cistern.glb'),export_format='GLB',export_yup=True,export_texcoords=True,export_normals=True,export_materials='EXPORT',export_cameras=False,export_lights=False,export_extras=True)
meshes=[o for o in scene.objects if o.type=='MESH'];tris=sum(sum(len(p.vertices)-2 for p in o.data.polygons) for o in meshes)
meta={'scene':'cistern/cistern.glb#Scene0','water_y':-.22,'floor_y':0,'lights':LAMPS,'nodes':{'tool_lid':{'hinge_world':[-6,.55,-.29],'open_local_rotation_x':-1.7},'tonic_lid':{'hinge_world':[-5,.55,-4.29],'open_local_rotation_x':-1.7},'wheel_pickup':{'position':[6,.86,-4],'hide_descendants':True}},'wall_height_rule':['z>=4: .48','x==-10 or x>=9: 1.15','south glyph != #: 2.15','otherwise .74'],'wall_footprint':'integer cell +/- .5; carved north facade columns may reach 2.7','budget':{'mesh_objects':len(meshes),'triangles':tris,'glb_bytes':(OUT/'cistern.glb').stat().st_size},'grid_snapshot':GRID}
(ART/'integration.json').write_text(json.dumps(meta,ensure_ascii=False,indent=2)+'\n')
(OUT/'lights.json').write_text(json.dumps([{'pos':[l['position'][0],round(l['position'][1]+(0 if l.get('type') else .09),3),l['position'][2]],'kind':{'crystal':'crystal','seep':'leak'}.get(l.get('type'),'brazier')} for l in LAMPS],indent=2)+'\n')
# Real world-space rendered previews; no composited image backdrop or light painting.
scene.render.engine='CYCLES';scene.cycles.samples=32;scene.cycles.use_denoising=True
scene.world.color=(.13,.16,.18)
world=scene.world;world.use_nodes=True;world.node_tree.nodes['Background'].inputs['Color'].default_value=(.14,.19,.24,1);world.node_tree.nodes['Background'].inputs['Strength'].default_value=.16
scene.view_settings.view_transform='AgX';scene.view_settings.look='AgX - Medium High Contrast';scene.view_settings.exposure=.25
# Wide faint overhead area light provides legibility without erasing intrinsic recesses.
def light(name,kind,p,color,power,size=3):
    d=bpy.data.lights.new(name,kind);d.color=color;d.energy=power
    if kind=='AREA':d.shape='DISK';d.size=size
    else:d.shadow_soft_size=.23
    o=bpy.data.objects.new(name,d);bpy.context.collection.objects.link(o);o.location=cv(p)
    return o
key=light('Preview cold ceiling bounce','AREA',(0,9,-1),(.57,.70,.83),650,18)
for l in LAMPS:light(l['name'],'POINT',l['position'],l['color'],l['lumens']*.10)
fill=light('Preview soft south fill','AREA',(-3,6,7),(.62,.66,.68),380,12)
fill.rotation_euler=(Vector(cv((0,0,-1)))-fill.location).to_track_quat('-Z','Y').to_euler()
camdata=bpy.data.cameras.new('North-facing gameplay preview');cam=bpy.data.objects.new('North-facing gameplay preview',camdata);bpy.context.collection.objects.link(cam);scene.camera=cam
camdata.type='PERSP';camdata.sensor_fit='VERTICAL';camdata.sensor_height=24;camdata.lens=12/math.tan(math.radians(17))
scene.render.resolution_x=1600;scene.render.resolution_y=1100;scene.render.resolution_percentage=100
# Exact zero-yaw straight north. Game default uses 56 degrees downward and 27m distance.
def render(name,target,distance,pitch):
    pitch=math.radians(pitch);eye=(target[0],target[1]+distance*math.sin(pitch),target[2]+distance*math.cos(pitch))
    cam.location=cv(eye);cam.rotation_euler=(Vector(cv(target))-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=str(ART/name);bpy.ops.render.render(write_still=True)
# Source has preview lights/camera for editable art inspection, never exported into game GLB.
render('preview_north_overview.png',(0,.5,-.1),27,56)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(ART/'cistern.blend'))
scene.render.resolution_x=1400;scene.render.resolution_y=1000
render('preview_north_near.png',(-.4,.45,-3.0),11.5,56)
render('preview_east_near.png',(5.7,.42,-2.7),9.0,56)
for name in ['tool_lid','tonic_lid']:bpy.data.objects[name].rotation_euler.x=-1.7
render('preview_coffers_open.png',(-5.5,.36,-2.0),8.5,56)
print('CISTERN ART COMPLETE',json.dumps(meta['budget']))
