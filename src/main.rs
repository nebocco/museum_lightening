#[cfg(target_arch = "wasm32")]
use bevy::ecs as bevy_ecs;
use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::mesh::Indices,
    render::render_resource::PrimitiveTopology,
    sprite::{collide_aabb, MaterialMesh2dBundle},
    window::PrimaryWindow,
};
use geo::algorithm::triangulate_earcut::TriangulateEarcut;
use geo::{ConvexHull, Intersects, Line, LineString, MultiPoint, MultiPolygon, Polygon};

mod geo_scaled;
use geo_scaled::ScaledBooleanOps;

const COLOR_NORMAL: Color = Color::ALICE_BLUE;
const COLOR_SHADOW: Color = Color::GRAY;
const COLOR_SHADOW_UNION: Color = Color::SILVER;
const COLOR_SHADOW_INTERSECTION: Color = Color::GRAY;
const COLOR_LIGHT: Color = Color::FUCHSIA;
const COLOR_OBSTACLE: Color = Color::DARK_GRAY;

const WORLD_WIDTH: f32 = 960.0;
const WORLD_HEIGHT: f32 = 720.0;

const LIGHT_SIZE: f32 = 10.0;

const LIGHT_Z: f32 = 3.0;
const OBSTACLE_Z: f32 = 2.0;
const DARK_SHADOW_Z: f32 = 1.0;
const PALE_SHADOW_Z: f32 = 0.5;
const BACKGROUND_Z: f32 = 0.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy museum".to_string(),
                resolution: (1024.0, 768.0).into(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .insert_resource(ClearColor(COLOR_SHADOW))
        .insert_resource(WorldScale(1.0))
        .init_resource::<WorldCoords>()
        .add_event::<MouseMotion>()
        .add_systems(Startup, setup)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, (grab_object, drag_object, drop_object))
        .add_systems(
            Update,
            (
                change_camera_scale,
                scale_world_with_scroll,
                zoom_reset,
                screen_move,
            ),
        )
        .add_systems(Update, cursor_position_to_world_coordinate)
        .add_systems(Update, update)
        .run();
}

#[derive(Component)]
struct CameraLabel;

#[derive(Component)]
struct Light;

#[derive(Component)]
struct Obstacle;

#[derive(Component)]
struct Shadow;

#[derive(Component)]
struct Theta(f32, f32);

#[derive(Component)]
struct Draggable;

#[derive(Component)]
struct Dragging;

#[derive(Resource, Default)]
struct WorldCoords(Vec2);

#[derive(Resource)]
struct WorldScale(f32);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(CameraLabel);

    // World
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: COLOR_NORMAL,
            custom_size: Some(Vec2::new(WORLD_WIDTH, WORLD_HEIGHT)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, BACKGROUND_Z),
        ..Default::default()
    },));

    // Circle
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(1.0).into()).into(),
            material: materials.add(ColorMaterial::from(COLOR_LIGHT)),
            transform: Transform::from_translation(Vec3::new(400.0, 0.0, LIGHT_Z))
                .with_scale(Vec3::new(LIGHT_SIZE, LIGHT_SIZE, 1.0)),
            ..default()
        },
        Light,
        Theta(0.0, 0.40),
        Draggable,
    ));

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(1.0).into()).into(),
            material: materials.add(ColorMaterial::from(COLOR_LIGHT)),
            transform: Transform::from_translation(Vec3::new(-400.0, 0.0, LIGHT_Z))
                .with_scale(Vec3::new(LIGHT_SIZE, LIGHT_SIZE, 1.0)),
            ..default()
        },
        Light,
        Theta(std::f32::consts::FRAC_PI_3, -0.35),
        Draggable,
    ));

    // Quad
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: COLOR_OBSTACLE,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, -200.0, OBSTACLE_Z))
                .with_scale(Vec3::new(60.0, 100.0, 1.0))
                .with_rotation(Quat::from_rotation_z(0.0_f32.to_radians())),
            ..default()
        },
        Obstacle,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: COLOR_OBSTACLE,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-50.0, 50.0, OBSTACLE_Z))
                .with_scale(Vec3::new(10.0, 300.0, 1.0))
                .with_rotation(Quat::from_rotation_z(-60.0_f32.to_radians())),
            ..default()
        },
        Obstacle,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: COLOR_OBSTACLE,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-350.0, -250.0, OBSTACLE_Z))
                .with_scale(Vec3::new(20.0, 70.0, 1.0))
                .with_rotation(Quat::from_rotation_z(-45.0_f32.to_radians())),
            ..default()
        },
        Obstacle,
    ));
}

fn grab_object(
    mut commands: Commands,
    draggable: Query<(Entity, &Transform), With<Draggable>>,
    dragging: Query<&Dragging>,
    mouse_button: Res<Input<MouseButton>>,
    cursor_position: Res<WorldCoords>,
) {
    if dragging.get_single().is_ok() || !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }
    for (e, transform) in draggable.iter() {
        if collide_aabb::collide(
            cursor_position.0.extend(0.0),
            [0.0, 0.0].into(),
            transform.translation,
            transform.scale.truncate() * 2.0,
        )
        .is_some()
        {
            commands.entity(e).insert(Dragging);
            return;
        }
    }
}

fn drag_object(
    mut object: Query<&mut Transform, With<Dragging>>,
    mouse_button: Res<Input<MouseButton>>,
    cursor_position: Res<WorldCoords>,
) {
    if !mouse_button.pressed(MouseButton::Left) {
        return;
    }
    let Ok(mut transform) = object.get_single_mut() else {
        return;
    };
    transform.translation = cursor_position.0.extend(transform.translation.z);
}

fn drop_object(
    mut commands: Commands,
    object: Query<Entity, With<Dragging>>,
    mouse_button: Res<Input<MouseButton>>,
) {
    if mouse_button.just_released(MouseButton::Left) {
        if let Ok(e) = object.get_single() {
            commands.entity(e).remove::<Dragging>();
        }
    }
}

fn cursor_position_to_world_coordinate(
    mut mycoords: ResMut<WorldCoords>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<CameraLabel>>,
) {
    let (camera, camera_transform) = q_camera.single();
    let window = q_window.single();
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        mycoords.0 = world_position;
    }
}

fn update(
    mut commands: Commands,
    shadows: Query<Entity, With<Shadow>>,
    lights: Query<&Transform, With<Light>>,
    obstacles: Query<&Transform, With<Obstacle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for entity in shadows.iter() {
        commands.entity(entity).despawn();
    }

    let mut shadow_polygons = Vec::new();
    for light in lights.iter() {
        let shadow_polygon = obstacles
            .iter()
            .map(|obstacle| {
                calculate_shadow_polygon_from_obstacle(
                    light.translation.truncate(),
                    obstacle,
                    (
                        Vec2::new(-WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
                        Vec2::new(WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
                    ),
                )
            })
            .fold(MultiPolygon::new(Vec::new()), |fold, polygon| {
                fold.scaled_union(&MultiPolygon::new(vec![polygon]), 1e1)
            });

        shadow_polygons.push(shadow_polygon);
    }
    let shadow_polygon_union = shadow_polygons
        .iter()
        .fold(MultiPolygon::new(Vec::new()), |fold, polygon| {
            fold.scaled_union(polygon, 1e1)
        });
    let shadow_polygon_intersection = shadow_polygons
        .into_iter()
        .reduce(|fold, polygon| fold.scaled_intersection(&polygon, 1e1))
        .unwrap();

    for shadow in shadow_polygon_union.into_iter() {
        let (translation, mesh) = create_polygon_mesh(&shadow);
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(mesh).into(),
                material: materials.add(ColorMaterial::from(COLOR_SHADOW_UNION)),
                transform: Transform::from_translation(translation.extend(PALE_SHADOW_Z)),
                ..Default::default()
            },
            Shadow,
        ));
    }
    for shadow in shadow_polygon_intersection.into_iter() {
        let (translation, mesh) = create_polygon_mesh(&shadow);
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(mesh).into(),
                material: materials.add(ColorMaterial::from(COLOR_SHADOW_INTERSECTION)),
                transform: Transform::from_translation(translation.extend(DARK_SHADOW_Z)),
                ..Default::default()
            },
            Shadow,
        ));
    }
}

fn calculate_vertices(transform: &Transform) -> [Vec2; 4] {
    let rotation = Vec2::from_angle(transform.rotation.to_euler(EulerRot::YXZ).2);
    let size = transform.scale;
    let translation = transform.translation.truncate();
    let res = [
        rotation.rotate(Vec2::new(-size.x / 2., -size.y / 2.)) + translation,
        rotation.rotate(Vec2::new(size.x / 2., -size.y / 2.)) + translation,
        rotation.rotate(Vec2::new(size.x / 2., size.y / 2.)) + translation,
        rotation.rotate(Vec2::new(-size.x / 2., size.y / 2.)) + translation,
    ];

    return res;
}

fn calculate_intersection_to_world_bondary(
    u: Vec2,
    v: Vec2,
    world_boundary: &(Vec2, Vec2),
) -> Vec2 {
    let ray = v - u;

    // 横の衝突
    let s = if ray.x < 0.0 {
        (world_boundary.0.x - u.x) / ray.x
    } else {
        (world_boundary.1.x - u.x) / ray.x
    };

    // 縦の衝突
    let t = if ray.y < 0.0 {
        (world_boundary.0.y - u.y) / ray.y
    } else {
        (world_boundary.1.y - u.y) / ray.y
    };
    if s < t {
        if ray.x < 0.0 {
            Vec2::new(world_boundary.0.x, u.y + ray.y * s)
        } else {
            Vec2::new(world_boundary.1.x, u.y + ray.y * s)
        }
    } else {
        if ray.y < 0.0 {
            Vec2::new(u.x + ray.x * t, world_boundary.0.y)
        } else {
            Vec2::new(u.x + ray.x * t, world_boundary.1.y)
        }
    }
}

fn calculate_shadow_polygon_from_obstacle(
    light_position: Vec2,
    obstacle_transform: &Transform,
    world_boundary: (Vec2, Vec2),
) -> Polygon<f32> {
    const WORLD_VERTICES: [Vec2; 4] = [
        Vec2::new(WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
        Vec2::new(-WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
        Vec2::new(-WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
        Vec2::new(WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
    ];

    let obstacle_vertices = calculate_vertices(obstacle_transform);
    let obstacle_polygon = Polygon::<f32>::new(
        LineString::from_iter(obstacle_vertices.iter().map(|v| v.to_array())),
        Vec::new(),
    );

    let multi_points = MultiPoint::from_iter(
        obstacle_vertices
            .iter()
            .map(|v| v.to_array())
            // 壁との交点
            .chain(obstacle_vertices.iter().map(|&v| {
                calculate_intersection_to_world_bondary(light_position, v, &world_boundary)
                    .to_array()
            }))
            // 死角となっている四隅
            .chain(
                WORLD_VERTICES
                    .iter()
                    .copied()
                    .filter(|v| {
                        obstacle_polygon
                            .intersects(&Line::new(light_position.to_array(), v.to_array()))
                    })
                    .map(|v| v.to_array()),
            ),
    );

    multi_points.convex_hull()
}

fn create_polygon_mesh(polygon: &Polygon<f32>) -> (Vec2, Mesh) {
    // 頂点のリストを取得
    let mut vertices = Vec::new();
    let triangle_list = polygon.earcut_triangles();
    for tri in &triangle_list {
        let arr = tri.to_array();
        for v in arr {
            vertices.push(v);
        }
    }
    vertices.sort_by(|u, v| u.x.total_cmp(&v.x).then_with(|| u.y.total_cmp(&v.y)));
    vertices.dedup();

    let translation = Vec2::new(vertices[0].x, vertices[0].y);
    let vertices_vec3: Vec<Vec3> = vertices
        .iter()
        .map(|&v| {
            let u = v - vertices[0];
            Vec3::new(u.x, u.y, 0.0)
        })
        .collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices_vec3);
    // Assign a UV coordinate to each vertex.
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; vertices.len()]);
    // Assign normals (everything points outwards)
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; vertices.len()],
    );

    let triangle_indices: Vec<u32> = triangle_list
        .iter()
        .map(|t| {
            let [v0, v1, v2] = t.to_array();
            [
                vertices.iter().position(|&v| v == v0).unwrap() as u32,
                vertices.iter().position(|&v| v == v1).unwrap() as u32,
                vertices.iter().position(|&v| v == v2).unwrap() as u32,
            ]
        })
        .flatten()
        .collect();

    mesh.set_indices(Some(Indices::U32(triangle_indices)));

    (translation, mesh)
}

fn scale_world_with_scroll(
    mut scroll_evr: EventReader<MouseWheel>,
    mut world_scale: ResMut<WorldScale>,
) {
    if scroll_evr.is_empty() {
        return;
    }
    for ev in scroll_evr.iter() {
        if ev.y > 0.0 {
            world_scale.0 *= 0.85;
        } else if ev.y < 0.0 {
            world_scale.0 *= 1.15;
        }
    }
    world_scale.0 = world_scale.0.clamp(0.2, 3.0);
}

fn zoom_reset(
    keys: Res<Input<KeyCode>>,
    mut world_scale: ResMut<WorldScale>,
    mut query: Query<&mut Transform, With<CameraLabel>>,
) {
    if keys.just_pressed(KeyCode::Key0) {
        let mut transform = query.single_mut();
        transform.translation.x = 0.0;
        transform.translation.y = 0.0;
        world_scale.0 = 1.0;
    }
}

fn change_camera_scale(
    world_scale: Res<WorldScale>,
    mut query: Query<&mut OrthographicProjection, With<CameraLabel>>,
) {
    if world_scale.is_changed() {
        let mut camera = query.single_mut();
        camera.scale = world_scale.0;
    }
}

fn screen_move(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,

    mut query: Query<&mut Transform, With<CameraLabel>>,
) {
    const SPEED: f32 = WORLD_WIDTH / 2.0;

    let mut camera = query.single_mut();
    if keys.pressed(KeyCode::Right) {
        camera.translation.x += SPEED * time.delta_seconds();
    }
    if keys.pressed(KeyCode::Left) {
        camera.translation.x -= SPEED * time.delta_seconds();
    }
    if keys.pressed(KeyCode::Up) {
        camera.translation.y += SPEED * time.delta_seconds();
    }
    if keys.pressed(KeyCode::Down) {
        camera.translation.y -= SPEED * time.delta_seconds();
    }

    camera.translation.x = camera
        .translation
        .x
        .clamp(-WORLD_WIDTH / 2., WORLD_WIDTH / 2.);
    camera.translation.y = camera
        .translation
        .y
        .clamp(-WORLD_HEIGHT / 2., WORLD_HEIGHT / 2.);
}
