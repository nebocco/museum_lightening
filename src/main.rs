use bevy::{
    prelude::*, render::mesh::Indices, render::render_resource::PrimitiveTopology,
    sprite::MaterialMesh2dBundle,
};
use geo::algorithm::triangulate_earcut::TriangulateEarcut;
use geo::{ConvexHull, Intersects, Line, LineString, MultiPoint, Polygon};

const COLOR_NORMAL: Color = Color::ALICE_BLUE;
const COLOR_SHADOW: Color = Color::GRAY;
const COLOR_LIGHT: Color = Color::GOLD;
const COLOR_OBSTACLE: Color = Color::DARK_GRAY;

const WORLD_WIDTH: f32 = 960.0;
const WORLD_HEIGHT: f32 = 720.0;

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
        .add_systems(Startup, setup)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Update, (update, move_objects))
        .run();
}

#[derive(Component)]
struct Light;

#[derive(Component)]
struct Obstacle;

#[derive(Component)]
struct Shadow;

#[derive(Component)]
struct Theta(f32);

const LIGHT_SIZE: f32 = 10.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    // World
    commands.spawn((SpriteBundle {
        sprite: Sprite {
            color: COLOR_NORMAL,
            custom_size: Some(Vec2::new(WORLD_WIDTH, WORLD_HEIGHT)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..Default::default()
    },));

    // Circle
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(LIGHT_SIZE).into()).into(),
            material: materials.add(ColorMaterial::from(COLOR_LIGHT)),
            transform: Transform::from_translation(Vec3::new(400., 0., 1.)),
            ..default()
        },
        Light,
        Theta(0.0),
    ));

    // Quad
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: COLOR_OBSTACLE,
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., -200., 1.))
                .with_rotation(Quat::from_rotation_z(0.0_f32.to_radians())),
            ..default()
        },
        Obstacle,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: COLOR_OBSTACLE,
                custom_size: Some(Vec2::new(30.0, 300.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-50., 50., 1.))
                .with_rotation(Quat::from_rotation_z(-60.0_f32.to_radians())),
            ..default()
        },
        Obstacle,
    ));
}

fn move_objects(mut moving_objects: Query<(&mut Theta, &mut Transform)>, time: Res<Time>) {
    const D_THETA: f32 = 1.0;
    for (mut th, mut trans) in moving_objects.iter_mut() {
        th.0 += D_THETA * time.delta_seconds();
        if th.0 >= std::f32::consts::TAU {
            th.0 -= std::f32::consts::TAU;
        }
        trans.translation = Vec3::new(400.0 * th.0.cos(), 300.0 * th.0.sin(), 1.0);
    }
}

fn update(
    mut commands: Commands,
    shadows: Query<Entity, With<Shadow>>,
    lights: Query<&Transform, With<Light>>,
    obstacles: Query<(&Transform, &Sprite), With<Obstacle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for entity in shadows.iter() {
        commands.entity(entity).despawn();
    }

    for light in lights.iter() {
        for obstacle in obstacles.iter() {
            let shadow_polygon = calculate_shadow_polygon_from_obstacle(
                light.translation.truncate(),
                obstacle.1.custom_size.unwrap(),
                obstacle.0,
                (
                    Vec2::new(-WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
                    Vec2::new(WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
                ),
            );
            let (translation, mesh) = create_polygon_mesh(&shadow_polygon);

            commands.spawn((
                MaterialMesh2dBundle {
                    mesh: meshes.add(mesh).into(),
                    material: materials.add(ColorMaterial::from(COLOR_SHADOW)),
                    transform: Transform::from_translation(translation.extend(0.0)),
                    ..Default::default()
                },
                Shadow,
            ));
        }
    }
}

fn calculate_vertices(size: Vec2, transform: &Transform) -> [Vec2; 4] {
    let rotation =
        Vec2::from_angle(transform.rotation.to_euler(EulerRot::YXZ).2) * transform.scale.truncate();
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
    obstacle_size: Vec2,
    obstacle_transform: &Transform,
    world_boundary: (Vec2, Vec2),
) -> Polygon<f32> {
    const WORLD_VERTICES: [Vec2; 4] = [
        Vec2::new(WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
        Vec2::new(-WORLD_WIDTH / 2., WORLD_HEIGHT / 2.),
        Vec2::new(-WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
        Vec2::new(WORLD_WIDTH / 2., -WORLD_HEIGHT / 2.),
    ];

    let obstacle_vertices = calculate_vertices(obstacle_size, obstacle_transform);
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