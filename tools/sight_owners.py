"""Authoring contract for all camera-occluding models. ID 0 is permanent terrain.
TEXCOORD_1.x carries exactly one integer per face, including after material batching.
Bounds are solid boxes in Bevy world coordinates (X east, Y up, Z south).
"""
import json
from math import isfinite


class OwnerMap:
    def __init__(self, start, stop):
        self.start, self.stop = start, stop
        self.names = {}
        self.rows = []

    def add(self, name, cell=None):
        if name in self.names:
            raise ValueError(f'duplicate sight object: {name}')
        owner = self.start + len(self.rows)
        if not 0 < owner < self.stop <= 1024:
            raise ValueError(f'sight IDs exhausted: {name}, range {self.start}..{self.stop}')
        self.names[name] = owner
        self.rows.append(dict(id=owner, name=name, cell=cell, boxes=[]))
        return owner

    def box(self, owner, low_xyz_game, high_xyz_game):
        lo = [round(float(v), 5) for v in low_xyz_game]
        hi = [round(float(v), 5) for v in high_xyz_game]
        if len(lo) != 3 or len(hi) != 3 or not all(isfinite(a) and isfinite(b) and a <= b for a, b in zip(lo, hi)):
            raise ValueError(f'invalid sight bounds: {owner} {lo}..{hi}')
        self.rows[owner-self.start]['boxes'].append([lo, hi])

    @staticmethod
    def uv(mesh, face_ids):
        if len(mesh.uv_layers) != 1 or len(mesh.polygons) != len(face_ids):
            raise ValueError(f'{mesh.name}: need base UV and one owner per polygon')
        layer = mesh.uv_layers.new(name='SightOwner', do_init=False)
        for polygon, owner in zip(mesh.polygons, face_ids):
            for corner in polygon.loop_indices:
                layer.data[corner].uv = (float(owner), 0.5)
        mesh.uv_layers.active_index = 0
        mesh.uv_layers[0].active_render = True

    def write(self, path):
        empty = [r['name'] for r in self.rows if not r['boxes']]
        if empty:
            raise ValueError(f'sight objects missing solid bounds: {empty}')
        rows = [dict(r, boxes=self.compact(r['boxes'])) for r in self.rows]
        path.write_text(json.dumps(rows, separators=(',', ':')) + '\n')

    @staticmethod
    def compact(boxes):
        """Exact unions only: the open space under a roof or an arch stays open."""
        spans = set(tuple(lo + hi) for lo, hi in boxes)
        for axis in range(3):
            lines = {}
            for box in spans:
                key = tuple(box[i] for i in range(6) if i % 3 != axis)
                lines.setdefault(key, []).append((box[axis], box[axis+3]))
            spans = set()
            for key, intervals in lines.items():
                merged = []
                for lo, hi in sorted(intervals):
                    if merged and lo <= merged[-1][1]:
                        merged[-1][1] = max(hi, merged[-1][1])
                    else:
                        merged.append([lo, hi])
                for lo, hi in merged:
                    b = list(key); b.insert(axis, lo); b.insert(axis+3, hi)
                    spans.add(tuple(b))
        kept = []
        for b in sorted(spans, key=lambda t: (-(t[3]-t[0])*(t[4]-t[1])*(t[5]-t[2]), t)):
            if not any(k[0]<=b[0] and k[1]<=b[1] and k[2]<=b[2] and
                       k[3]>=b[3] and k[4]>=b[4] and k[5]>=b[5] for k in kept):
                kept.append(b)
        return [[b[:3], b[3:]] for b in kept]
