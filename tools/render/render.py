"""Blender Cycles render with Intel Open Image Denoise (Blender: GPL, separate process). Headless:
  blender -b --factory-startup --python tools/render/render.py -- <input.glb|input.blend> <out.png> <thumbnail|hero>

A .glb is placed in a neutral studio (grey world, key and fill lights) with a camera framed on its
bounding box. A .blend scene renders with its own camera and lighting. The result is a Rendered
preview (Derived), never engineering evidence. Prints `HELQIX_RENDER=<json>`.
"""
import json
import math
import os
import sys
import time

import bpy
from mathutils import Vector

args = sys.argv[sys.argv.index("--") + 1:]
src, out_png, mode = args[0], args[1], args[2]
PRESETS = {"thumbnail": (256, 256, 32), "hero": (1280, 720, 128)}
if mode not in PRESETS:
    raise SystemExit(f"mode must be one of {sorted(PRESETS)}")
w, h, samples = PRESETS[mode]
started = time.time()

if src.lower().endswith(".blend"):
    bpy.ops.wm.open_mainfile(filepath=src)
    sc = bpy.context.scene
    if sc.camera is None:
        raise SystemExit("the scene file has no camera")
else:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)
    sc = bpy.context.scene
    meshes = [o for o in sc.objects if o.type == "MESH"]
    if not meshes:
        raise SystemExit("the model contains no mesh")
    corners = [o.matrix_world @ Vector(c) for o in meshes for c in o.bound_box]
    lo = Vector((min(c.x for c in corners), min(c.y for c in corners), min(c.z for c in corners)))
    hi = Vector((max(c.x for c in corners), max(c.y for c in corners), max(c.z for c in corners)))
    centre, radius = (lo + hi) / 2, max((hi - lo).length / 2, 1e-6)
    world = bpy.data.worlds.new("studio")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.42, 0.43, 0.45, 1.0)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 1.0
    sc.world = world
    for name, direction, energy in (("key", Vector((1.0, -1.2, 1.4)), 4.5), ("fill", Vector((-1.4, -0.6, 0.6)), 1.5)):
        light = bpy.data.objects.new(name, bpy.data.lights.new(name, "SUN"))
        light.data.energy = energy
        light.rotation_euler = (-direction).to_track_quat("-Z", "Y").to_euler()
        sc.collection.objects.link(light)
    cam_data = bpy.data.cameras.new("camera")
    cam_data.lens = 50
    cam = bpy.data.objects.new("camera", cam_data)
    sc.collection.objects.link(cam)
    view = Vector((1.0, -1.3, 0.9)).normalized()
    fov = 2 * math.atan(cam_data.sensor_width / (2 * cam_data.lens)) * min(w, h) / max(w, h)
    cam.location = centre + view * (radius / math.sin(fov / 2)) * 1.15
    cam.rotation_euler = (centre - cam.location).to_track_quat("-Z", "Y").to_euler()
    cam_data.clip_end = (cam.location - centre).length + radius * 4
    sc.camera = cam

sc.render.engine = "CYCLES"
sc.cycles.device = "CPU"
sc.cycles.samples = samples
sc.cycles.seed = 0
sc.cycles.use_denoising = True
sc.cycles.denoiser = "OPENIMAGEDENOISE"
sc.render.resolution_x, sc.render.resolution_y, sc.render.resolution_percentage = w, h, 100
sc.render.image_settings.file_format = "PNG"
sc.render.filepath = out_png
t = time.time()
bpy.ops.render.render(write_still=True)
try:
    import _cycles
    oidn = bool(getattr(_cycles, "with_openimagedenoise", False))
except Exception:
    oidn = None
print("HELQIX_RENDER=" + json.dumps({
    "blender": bpy.app.version_string, "mode": mode, "width": w, "height": h, "samples": samples, "denoiser": sc.cycles.denoiser,
    "cycles_has_oidn": oidn, "render_seconds": round(time.time() - t, 3), "seconds": round(time.time() - started, 3),
    "bytes": os.path.getsize(out_png), "label": "Rendered preview",
}))
