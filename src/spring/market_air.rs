//! Finite, aperture-anchored town air. Presentation time never advances game time.
use super::*;

/// A tapered ribbon follows the sun axis; its width cannot rotate with the camera.
fn sun_ribbon(top: Vec3, toe: Vec3, head_width: f32, toe_width: f32) -> Mesh {
    let span = crate::daylight::SQUARE.to_sun().cross(Vec3::Y).normalize();
    let corners = [
        top - span * head_width,
        top + span * head_width,
        toe + span * toe_width,
        toe - span * toe_width,
    ];
    let normal = (toe - top).cross(span).normalize().to_array();
    mesh(
        corners.map(|v| v.to_array()).to_vec(),
        vec![normal; 4],
        vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        vec![0, 2, 1, 0, 3, 2, 0, 1, 2, 0, 2, 3],
    )
}

pub(super) fn spawn(
    c: &mut Commands,
    root: Entity,
    meshes: &mut Assets<Mesh>,
    quad: &Handle<Mesh>,
    air: &mut impl FnMut(Vec4) -> Handle<AirInk>,
) {
    let sun = crate::daylight::SQUARE.to_sun();
    // The gate vault is solid. Light spills over its real parapet crenel,
    // rather than appearing to penetrate the barrel vault or its rear sluice.
    let gate = Vec3::new(-1.56, 4.90, -6.30);
    // make_lane.py cuts a .13 m radius sun-axis aperture through every crown card.
    let crown_gap = Vec3::new(6.05, 2.95, -4.55);
    for (name, top, toe, head_width, toe_width, strength, phase) in [
        (
            "Sun spill over the water-gate crenel",
            gate,
            gate - sun * ((gate.y - 0.18) / sun.y),
            0.095,
            0.29,
            0.034,
            2.7,
        ),
        (
            "Sun through the orchard crown aperture",
            crown_gap + sun * 1.8,
            crown_gap - sun * ((crown_gap.y - 1.05) / sun.y),
            0.075,
            0.11,
            0.047,
            8.3,
        ),
    ] {
        c.spawn((
            Name::new(name),
            Mesh3d(meshes.add(sun_ribbon(top, toe, head_width, toe_width))),
            MeshMaterial3d(air(Vec4::new(8., phase, strength, 0.))),
            NotShadowCaster,
            ChildOf(root),
        ));
    }

    // Stationary, feathered pockets remain behind the arch and distant eaves.
    // They never follow the camera across the playable square.
    for (name, p, size, strength) in [
        (
            "Cool air inside the water-gate recess",
            Vec3::new(-0.25, 2.12, -8.15),
            Vec2::new(3.25, 2.65),
            0.026,
        ),
        (
            "Air between the western distant eaves",
            Vec3::new(-5.6, 5.35, -8.10),
            Vec2::new(3.2, 1.25),
            0.019,
        ),
        (
            "Air between the eastern distant eaves",
            Vec3::new(5.1, 5.45, -8.10),
            Vec2::new(3.0, 1.15),
            0.017,
        ),
    ] {
        c.spawn((
            Name::new(name),
            Mesh3d(quad.clone()),
            MeshMaterial3d(air(Vec4::new(9., 0., strength, 0.))),
            Transform::from_translation(p).with_scale(size.extend(1.)),
            NotShadowCaster,
            ChildOf(root),
        ));
    }

    // These are blossom trees, not the green fruit trees at (5,-4)/(8,-2).
    // Each small card holds one petal or one briefly lit mote, with separate phases.
    for (tree, scale, phase) in [
        (Vec3::new(-10.5, 0., 5.5), 1.4, 1.3),
        (Vec3::new(10., 0., 7.5), 1.3, 9.7),
    ] {
        for (offset, size, variant, delay) in [
            (Vec3::new(0.92, 2.06, -0.24), Vec2::new(0.65, 1.15), 0., 0.),
            (Vec3::new(0.47, 1.10, -0.70), Vec2::new(0.52, 0.92), 0., 5.3),
            (
                Vec3::new(-0.68, 1.59, 0.62),
                Vec2::new(0.56, 1.04),
                0.,
                12.6,
            ),
            (Vec3::new(0.74, 2.61, -0.43), Vec2::new(0.40, 0.48), 1., 7.9),
        ] {
            c.spawn((
                Name::new("A blossom drifting beside its tree"),
                AirCard,
                Mesh3d(quad.clone()),
                MeshMaterial3d(air(Vec4::new(10., phase + delay, 0.80, variant))),
                Transform::from_translation(tree + offset * scale)
                    .with_scale((size * scale).extend(1.)),
                NotShadowCaster,
                ChildOf(root),
            ));
        }
        // A few uneven, stationary fallen petals collect near the trunk and verge.
        // The road centre and fountain footprint receive none of this effect.
        let fallen = air(Vec4::new(10., phase, 0.82, 2.));
        for (x, z, size, angle) in [
            (0.61, -0.37, 0.31, 0.4),
            (0.81, -0.23, 0.35, 1.7),
            (0.49, -0.15, 0.27, 2.8),
            (-0.20, 0.59, 0.30, 1.1),
            (1.14, -0.69, 0.29, 2.0),
        ] {
            c.spawn((
                Name::new("Fallen blossom at its tree verge"),
                Mesh3d(quad.clone()),
                MeshMaterial3d(fallen.clone()),
                Transform::from_translation(tree + Vec3::new(x * scale, 0.024, z * scale))
                    .with_rotation(
                        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                            * Quat::from_rotation_z(angle),
                    )
                    .with_scale(Vec3::splat(size * scale)),
                NotShadowCaster,
                ChildOf(root),
            ));
        }
    }
}
