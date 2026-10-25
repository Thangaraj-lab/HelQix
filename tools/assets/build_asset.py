"""Blender (GPL, separate process) build step for one asset. Run headless:
  blender -b --factory-startup --python tools/assets/build_asset.py -- <asset.json> <out.glb> <work-dir>

Imports the source (OBJ, glTF/GLB or STL; metres, Z up), joins it into one mesh with outward normals,
applies the spec's PBR material (base colour, metallic, roughness), unwraps UVs, bakes ambient
occlusion with Cycles into an image, connects it to glTF's occlusion slot, and exports GLB without
compression (glTF-Transform compresses next). Prints one line `HELQIX_BUILD=<json>`.
"""
import json
import os
import sys
import time

import bpy

args = sys.argv[sys.argv.index("--") + 1:]
spec_path, out_glb, work = args[0], args[1], args[2]
spec = json.load(open(spec_path, encoding="utf-8"))
src = os.path.join(os.path.dirname(spec_path), spec["source"])
os.makedirs(work, exist_ok=True)
started = time.time()

bpy.ops.wm.read_factory_settings(use_empty=True)
ext = os.path.splitext(src)[1].lower()
if ext == ".obj":
    bpy.ops.wm.obj_import(filepath=src, forward_axis="Y", up_axis="Z")
elif ext in (".glb", ".gltf"):
    bpy.ops.import_scene.gltf(filepath=src)
elif ext == ".stl":
    bpy.ops.wm.stl_import(filepath=src, forward_axis="Y", up_axis="Z")
else:
    raise SystemExit(f"unsupported source format {ext}")

meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
if not meshes:
    raise SystemExit("the source contains no mesh")
bpy.ops.object.select_all(action="DESELECT")
for o in meshes:
    o.select_set(True)
bpy.context.view_layer.objects.active = meshes[0]
if len(meshes) > 1:
    bpy.ops.object.join()
obj = bpy.context.view_layer.objects.active
obj.name = spec["name"]
bpy.ops.object.mode_set(mode="EDIT")
bpy.ops.mesh.select_all(action="SELECT")
bpy.ops.mesh.normals_make_consistent(inside=False)
bpy.ops.uv.smart_project(island_margin=0.02)
bpy.ops.object.mode_set(mode="OBJECT")

m = spec["material"]
mat = bpy.data.materials.new(m["name"])
mat.use_nodes = True
nt = mat.node_tree
bsdf = nt.nodes["Principled BSDF"]
bsdf.inputs["Base Color"].default_value = (*m["base_color"], 1.0)
bsdf.inputs["Metallic"].default_value = m["metallic"]
bsdf.inputs["Roughness"].default_value = m["roughness"]
obj.data.materials.clear()
obj.data.materials.append(mat)

# Ambient occlusion bake (Cycles) into an image used only as glTF occlusion.
ao = spec.get("ao_bake")
ao_info = None
if ao:
    res = int(ao["resolution"])
    img = bpy.data.images.new(f"{spec['name']}_ao", width=res, height=res, alpha=False, float_buffer=False)
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = img
    tex.image.colorspace_settings.name = "Non-Color"
    for n in nt.nodes:
        n.select = False
    tex.select = True
    nt.nodes.active = tex
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = int(ao["samples"])
    sc.cycles.seed = 0
    t = time.time()
    bpy.ops.object.bake(type="AO", margin=4)
    ao_seconds = time.time() - t
    path = os.path.join(work, f"{spec['name']}_ao.png")
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    # glTF exporter convention: a node group named "glTF Material Output" with an "Occlusion" input.
    group = bpy.data.node_groups.get("glTF Material Output") or bpy.data.node_groups.new("glTF Material Output", "ShaderNodeTree")
    if "Occlusion" not in [s.name for s in group.interface.items_tree]:
        group.interface.new_socket(name="Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
    gnode = nt.nodes.new("ShaderNodeGroup")
    gnode.node_tree = group
    sep = nt.nodes.new("ShaderNodeSeparateColor")
    nt.links.new(tex.outputs["Color"], sep.inputs["Color"])
    nt.links.new(sep.outputs[0], gnode.inputs["Occlusion"])
    ao_info = {"resolution": res, "samples": int(ao["samples"]), "bake_seconds": round(ao_seconds, 3)}

bpy.ops.object.select_all(action="DESELECT")
obj.select_set(True)
bpy.ops.export_scene.gltf(filepath=out_glb, export_format="GLB", use_selection=True, export_yup=True, export_apply=True,
                          export_image_format="AUTO", export_materials="EXPORT")

try:
    import _cycles
    oidn = bool(getattr(_cycles, "with_openimagedenoise", False))
except Exception:
    oidn = None
triangles = sum(max(len(p.vertices) - 2, 0) for p in obj.data.polygons)
print("HELQIX_BUILD=" + json.dumps({
    "blender": bpy.app.version_string, "triangles": triangles, "ao": ao_info, "cycles_has_oidn": oidn,
    "seconds": round(time.time() - started, 3), "out": out_glb,
}))
