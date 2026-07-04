use vrcx_0_vr_overlay::{
    build_dummy_panel_scene, grab_follow_transform, ray_quad_intersection, recenter_transform,
    Color, DummyPanelAction, DummyPanelModel, HitRegion, OverlayQuadSize, OverlaySize,
    OverlaySurfaceId, OverlayTransform, Ray3, Rect, UvPoint,
};

#[test]
fn raycast_hits_quad_center_and_boundaries() {
    let transform = OverlayTransform::identity();
    let ray = Ray3::new([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]);
    let quad = OverlayQuadSize::new(0.8, 0.6);

    let hit = ray_quad_intersection(ray, transform, quad).expect("center hit");

    assert!((hit.distance - 1.0).abs() < 0.001);
    assert!((hit.uv.x - 0.5).abs() < 0.001);
    assert!((hit.uv.y - 0.5).abs() < 0.001);

    let edge_ray = Ray3::new([0.4, 0.3, 1.0], [0.0, 0.0, -1.0]);
    let edge = ray_quad_intersection(edge_ray, transform, quad).expect("edge hit");
    assert!((edge.uv.x - 1.0).abs() < 0.001);
    assert!(edge.uv.y.abs() < 0.001);
}

#[test]
fn raycast_rejects_backface_and_misses() {
    let transform = OverlayTransform::identity();
    let quad = OverlayQuadSize::new(0.8, 0.6);

    assert!(ray_quad_intersection(
        Ray3::new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0]),
        transform,
        quad,
    )
    .is_none());
    assert!(ray_quad_intersection(
        Ray3::new([0.9, 0.0, 1.0], [0.0, 0.0, -1.0]),
        transform,
        quad,
    )
    .is_none());
}

#[test]
fn recenter_transform_places_panel_in_front_of_hmd() {
    let hmd = OverlayTransform::from_translation_rotation(
        [2.0, 1.5, -3.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );

    let panel = recenter_transform(hmd, 1.25, -0.15);

    assert!((panel.translation[0] - 2.0).abs() < 0.001);
    assert!((panel.translation[1] - 1.35).abs() < 0.001);
    assert!((panel.translation[2] - -4.25).abs() < 0.001);
    assert_eq!(panel.rotation, hmd.rotation);
}

#[test]
fn grab_follow_transform_preserves_controller_to_panel_offset() {
    let panel = OverlayTransform::from_translation([0.0, 1.0, -1.0]);
    let grab_start = OverlayTransform::from_translation([0.2, 0.9, -0.8]);
    let grab_move = OverlayTransform::from_translation([0.4, 1.1, -1.2]);

    let next_panel = grab_follow_transform(panel, grab_start, grab_move);

    assert!((next_panel.translation[0] - 0.2).abs() < 0.001);
    assert!((next_panel.translation[1] - 1.2).abs() < 0.001);
    assert!((next_panel.translation[2] - -1.4).abs() < 0.001);
}

#[test]
fn hit_region_consumes_uv_coordinates() {
    let region = HitRegion {
        id: "button:primary".to_string(),
        rect: Rect::new(25.0, 25.0, 50.0, 50.0),
    };
    let size = OverlaySize::new(100, 100);

    assert!(region.contains_uv(size, UvPoint::new(0.5, 0.5)));
    assert!(region.contains_uv(size, UvPoint::new(0.25, 0.25)));
    assert!(!region.contains_uv(size, UvPoint::new(0.9, 0.9)));
    assert!(!region.contains_uv(size, UvPoint::new(-1.0, -1.0)));
    assert!(!region.contains_uv(size, UvPoint::new(1.5, 0.5)));
}

#[test]
fn dummy_panel_scene_emits_stable_hit_regions() {
    let model = DummyPanelModel::default();
    let scene = build_dummy_panel_scene(&model);
    let region_ids: Vec<&str> = scene
        .hit_regions
        .iter()
        .map(|region| region.id.as_str())
        .collect();

    assert_eq!(scene.surface_id, OverlaySurfaceId::new("interactive-dummy"));
    assert_eq!(scene.size, OverlaySize::new(768, 576));
    assert!(region_ids.contains(&"button:primary"));
    assert!(region_ids.contains(&"button:secondary"));
    assert!(region_ids.contains(&"list"));
}

#[test]
fn dummy_panel_updates_hover_press_and_scroll_state() {
    let mut model = DummyPanelModel::default();
    let size = model.size;
    let scene = build_dummy_panel_scene(&model);
    let primary_uv = scene
        .hit_regions
        .iter()
        .find(|region| region.id == "button:primary")
        .map(|region| region.rect.center_uv(size))
        .expect("primary button region");

    let hovered = model.apply_uv_action(primary_uv, DummyPanelAction::Hover);
    assert_eq!(hovered.as_deref(), Some("button:primary"));
    assert_eq!(model.hovered_region_id.as_deref(), Some("button:primary"));

    let pressed = model.apply_uv_action(primary_uv, DummyPanelAction::ClickDown);
    assert_eq!(pressed.as_deref(), Some("button:primary"));
    assert_eq!(model.pressed_region_id.as_deref(), Some("button:primary"));
    assert_eq!(model.primary_click_count, 0);

    let released = model.apply_uv_action(primary_uv, DummyPanelAction::ClickUp);
    assert_eq!(released.as_deref(), Some("button:primary"));
    assert_eq!(model.pressed_region_id, None);
    assert_eq!(model.primary_click_count, 1);

    model.apply_uv_action(
        UvPoint::new(0.5, 0.5),
        DummyPanelAction::Scroll { delta: 10.0 },
    );
    assert_eq!(model.scroll_offset_rows, model.max_scroll_offset_rows());

    model.apply_uv_action(
        UvPoint::new(0.5, 0.5),
        DummyPanelAction::Scroll { delta: -100.0 },
    );
    assert_eq!(model.scroll_offset_rows, 0);

    let scene_after = build_dummy_panel_scene(&model);
    assert!(scene_after.commands.iter().any(|command| {
        matches!(
            command,
            vrcx_0_vr_overlay::DrawCommand::FillRect { color, .. }
                if color == &Color::rgba(14, 116, 144, 255)
        )
    }));
}
