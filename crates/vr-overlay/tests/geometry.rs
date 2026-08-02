use vrcx_0_vr_overlay::{
    grab_follow_transform, grab_follow_transform_facing, ray_quad_intersection, recenter_transform,
    OverlayQuadSize, OverlaySize, OverlayTransform, Ray3, Rect,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn rect_contains_edges_and_converts_its_center_to_uv() {
    let rect = Rect::new(20.0, 10.0, 40.0, 20.0);

    assert!(rect.contains_point(20.0, 10.0));
    assert!(rect.contains_point(60.0, 30.0));
    assert!(!rect.contains_point(60.1, 30.0));

    let center = rect.center_uv(OverlaySize::new(100, 50));
    assert_close(center.x, 0.4);
    assert_close(center.y, 0.4);
}

#[test]
fn ray_quad_intersection_maps_center_and_edges_to_uv() {
    let transform = OverlayTransform::identity();
    let quad = OverlayQuadSize::new(2.0, 2.0);

    let center = ray_quad_intersection(
        Ray3::new([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]),
        transform,
        quad,
    )
    .expect("front-facing center ray should hit");
    assert_close(center.uv.x, 0.5);
    assert_close(center.uv.y, 0.5);
    assert_close(center.distance, 1.0);

    let corner = ray_quad_intersection(
        Ray3::new([1.0, 1.0, 1.0], [0.0, 0.0, -1.0]),
        transform,
        quad,
    )
    .expect("quad edge should remain interactive");
    assert_close(corner.uv.x, 1.0);
    assert_close(corner.uv.y, 0.0);

    assert!(ray_quad_intersection(
        Ray3::new([1.001, 0.0, 1.0], [0.0, 0.0, -1.0]),
        transform,
        quad,
    )
    .is_none());
}

#[test]
fn ray_quad_intersection_rejects_parallel_back_facing_and_behind_rays() {
    let transform = OverlayTransform::identity();
    let quad = OverlayQuadSize::new(2.0, 2.0);

    assert!(
        ray_quad_intersection(Ray3::new([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]), transform, quad,)
            .is_none()
    );
    assert!(ray_quad_intersection(
        Ray3::new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0]),
        transform,
        quad,
    )
    .is_none());
    assert!(ray_quad_intersection(
        Ray3::new([0.0, 0.0, -1.0], [0.0, 0.0, -1.0]),
        transform,
        quad,
    )
    .is_none());
}

#[test]
fn ray_quad_intersection_supports_rotated_panels_and_zero_direction_fallback() {
    let rotated = OverlayTransform::from_translation_rotation(
        [0.0, 0.0, 0.0],
        [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
    );
    let quad = OverlayQuadSize::new(2.0, 2.0);

    let rotated_hit =
        ray_quad_intersection(Ray3::new([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]), rotated, quad)
            .expect("ray should hit a panel rotated around the Y axis");
    assert_close(rotated_hit.uv.x, 0.5);
    assert_close(rotated_hit.uv.y, 0.5);

    let fallback_hit = ray_quad_intersection(
        Ray3::new([0.0, 0.0, 1.0], [0.0, 0.0, 0.0]),
        OverlayTransform::identity(),
        quad,
    )
    .expect("zero direction should use the default forward ray");
    assert_close(fallback_hit.distance, 1.0);
}

#[test]
fn recenter_and_grab_follow_preserve_expected_translation_and_rotation() {
    let hmd = OverlayTransform::from_translation([1.0, 2.0, 3.0]);
    let recentered = recenter_transform(hmd, 2.0, 0.5);
    assert_eq!(recentered.translation, [1.0, 2.5, 1.0]);
    assert_eq!(recentered.rotation, hmd.rotation);

    let panel = OverlayTransform::from_translation([2.0, 3.0, 4.0]);
    let controller_start = OverlayTransform::from_translation([1.0, 1.0, 1.0]);
    let controller_current = OverlayTransform::from_translation([3.0, 0.0, 5.0]);
    let followed = grab_follow_transform(panel, controller_start, controller_current);
    assert_eq!(followed.translation, [4.0, 2.0, 8.0]);
    assert_eq!(followed.rotation, panel.rotation);

    let facing = grab_follow_transform_facing(
        OverlayTransform::identity(),
        OverlayTransform::identity(),
        OverlayTransform::identity(),
        Some(OverlayTransform::from_translation([0.0, 0.0, 2.0])),
    );
    assert_eq!(facing, OverlayTransform::identity());
}
