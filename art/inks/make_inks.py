"""Original Wanderstead stage props. Run: blender-4.5 --background --python art/inks/make_inks.py
All geometry/materials are authored here; no external assets. Blender Z-up -> glTF Y-up.
The preview is an asset-workbench render, not game evidence.
"""
import bpy
import math
from pathlib import Path
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/inks"
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)


def material(name, color, roughness=.75, metal=0):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color, 1)
    m.use_nodes = True
    m.node_tree.nodes.clear()
    bsdf = m.node_tree.nodes.new('ShaderNodeBsdfPrincipled')
    output = m.node_tree.nodes.new('ShaderNodeOutputMaterial')
    m.node_tree.links.new(bsdf.outputs['BSDF'], output.inputs['Surface'])
    bsdf.inputs['Base Color'].default_value = (*color, 1)
    bsdf.inputs['Roughness'].default_value = roughness
    bsdf.inputs['Metallic'].default_value = metal
    return m


ceramic = material('Ink_Bowl_OatGlaze', (.69, .47, .25), .29)
rim = material('Ink_Bowl_CreamRim', (.91, .82, .62), .3)
cream = material('Ink_White_Curd', (.97, .935, .82), .88)
juice = material('Ink_Fresh_Plum_Juice', (.32, .004, .035), .23)
flesh = material('Ink_Fresh_Plum_Flesh', (.48, .005, .065), .35)
skin = material('Ink_Plum_Purple_Skin', (.115, .012, .11), .4)
wood = material('Ink_Spoon_Wood', (.26, .095, .034), .7)


def mesh(name, vertices, faces, mats, indices=None):
    data = bpy.data.meshes.new(name)
    data.from_pydata(vertices, [], faces)
    data.materials.clear()
    for m in mats:
        data.materials.append(m)
    ob = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(ob)
    if indices:
        for poly, index in zip(data.polygons, indices):
            poly.material_index = index
    return ob


def lathe(name, profile, mats, rim_rows=()):
    n = 40
    verts = [(r*math.cos(i*math.tau/n), r*math.sin(i*math.tau/n), z)
             for r, z in profile for i in range(n)]
    faces, indices = [], []
    for j in range(len(profile)-1):
        for i in range(n):
            a=j*n+i
            b=j*n+(i+1)%n
            faces.append((a, b, b+n, a+n))
            indices.append(1 if j in rim_rows else 0)
    return mesh(name, verts, faces, mats, indices)


def export(name, objects):
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects:
        ob.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    bpy.ops.export_scene.gltf(filepath=str(OUT / (name+'.glb')), export_format='GLB',
                              use_selection=True, export_yup=True,
                              export_materials='EXPORT', export_cameras=False,
                              export_lights=False)
    for ob in objects:
        ob.hide_render=True
    return objects


bowl=lathe('Breakfast_Bowl', [(0,-.01),(.115,-.01),(.13,.012),(.16,.03),
    (.215,.07),(.285,.14),(.321,.184),(.324,.194),(.312,.203),
    (.298,.19),(.276,.143),(.205,.079),(.13,.056),(0,.056)],
    [ceramic,rim], (6,7,8))
export('bowl',[bowl])

# The curd surface has hand-sized soft curds, not a smooth white plastic disk.
verts=[(0,0,0)]
n=40
for k in range(1,6):
    r=.277*k/5
    for i in range(n):
        a=i*math.tau/n
        z=.003*math.sin(i*2.4+k)*math.sin(k*1.8)+.003*(1-k/5)
        verts.append((r*math.cos(a),r*math.sin(a),z))
faces=[(0,1+i,1+(i+1)%n) for i in range(n)]
for k in range(4):
    for i in range(n):
        a=1+k*n+i
        b=1+k*n+(i+1)%n
        faces.append((a,a+n,b+n,b))
curd=mesh('White_Condensed_Curd',verts,faces,[cream])
export('curd',[curd])

# An irregular stain, geometrically separate and less than a millimetre above curd.
verts=[(0,0,.001)]
for i in range(32):
    a=i*math.tau/32
    r=.22*(.81+.10*math.sin(a*3)+.07*math.sin(a*5+1))
    verts.append((r*math.cos(a),r*math.sin(a),.0015))
stain=mesh('Fresh_Purple_Red_Juice',verts,[(0,1+i,1+(i+1)%32) for i in range(32)],[juice])
export('juice',[stain])

# One fruit = one visible cut half-pair. A dark skin border surrounds moist red flesh.
parts=[]
for side in (-1,1):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=10, ring_count=6, radius=1,
        location=(side*.035,0,.016))
    ob=bpy.context.object
    ob.name='Plum_Cut_Skin'
    ob.scale=(.043,.049,.027)
    ob.data.materials.append(skin)
    parts.append(ob)
    bpy.ops.mesh.primitive_uv_sphere_add(segments=10,ring_count=4,radius=1,
        location=(side*.035,0,.032))
    ob=bpy.context.object
    ob.name='Plum_Cut_Flesh'
    ob.scale=(.036,.043,.012)
    ob.data.materials.append(flesh)
    parts.append(ob)
export('plum-cut',parts)

# Wooden eating spoon, anchored at the bowl edge. Local handle points +X.
spoon=lathe('Wooden_Spoon_Cup',[(0,0),(.027,0),(.049,.009),(.052,.018),
    (.045,.018),(.024,.01),(0,.01)],[wood])
spoon.scale=(1.25,.8,1)
bpy.ops.mesh.primitive_cube_add(size=1,location=(.135,0,.021))
handle=bpy.context.object
handle.name='Wooden_Spoon_Handle'
handle.scale=(.24,.025,.022)
handle.rotation_euler[1]=-.075
handle.data.materials.append(wood)
export('spoon',[spoon,handle])

# Asset-maker inspection: arrange an actual serving and render it with real light/shadows.
for ob in [bowl,curd,stain,spoon,handle]+parts:
    ob.hide_render=False
curd.location.z=.153
stain.location.z=.159
spoon_rig=bpy.data.objects.new('Preview_Spoon_Rest',None)
bpy.context.collection.objects.link(spoon_rig)
spoon.parent=spoon_rig
handle.parent=spoon_rig
spoon_rig.location=(.14,-.11,.153)
spoon_rig.rotation_euler=(0,-.22,-.35)
for i in range(5):
    for source in parts:
        ob=source.copy()
        ob.data=source.data
        bpy.context.collection.objects.link(ob)
        angle=i*2.39996
        r=.07+.06*(i%3)
        ob.location=source.location+Vector((r*math.cos(angle),r*math.sin(angle),.158))
        ob.rotation_euler[2]=angle
        ob.hide_render=False
for ob in parts:
    ob.hide_render=True
bpy.ops.mesh.primitive_plane_add(size=200)
plane=bpy.context.object
plane.location.z=-.01
plane.data.materials.append(material('Preview_Table',(.26,.16,.08),.9))
bpy.ops.object.light_add(type='AREA',location=(-2,-3,5))
bpy.context.object.data.energy=450
bpy.context.object.data.shape='DISK'
bpy.context.object.data.size=3
bpy.ops.object.camera_add(location=(.75,-.95,.82))
cam=bpy.context.object
cam.rotation_euler=(Vector((0,0,.13))-cam.location).to_track_quat('-Z','Y').to_euler()
cam.data.type='ORTHO'
cam.data.ortho_scale=1.0
scene=bpy.context.scene
scene.camera=cam
scene.render.engine='CYCLES'
scene.cycles.samples=24
scene.world.color=(.18,.22,.29)
scene.render.resolution_x=640
scene.render.resolution_y=640
scene.render.resolution_percentage=100
scene.render.image_settings.file_format='PNG'
scene.render.filepath=str(ROOT/'art/inks/breakfast-workbench.png')
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'art/inks/breakfast-workbench.blend'))
bpy.ops.render.render(write_still=True)
