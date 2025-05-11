use std::f32::consts::TAU;

use bevy::{
    color::palettes::tailwind, gltf::{Gltf, GltfMesh, GltfNode}, math::Vec3Swizzles, prelude::*, render::camera::Exposure, window::CursorGrabMode
};
use bevy_mesh_decal::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy_fps_controller::controller::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 0.0);



fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 30000.0,
        })
        .insert_resource(SprayMaterials::default())
        .insert_resource(ClearColor(Color::linear_rgb(0.83, 0.96, 0.96)))
        .add_plugins(DefaultPlugins)
        .add_plugins(DecalPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (manage_cursor, scene_colliders, display_text, respawn, painter, make_all_decalable),
        )
        .add_systems(
            Last,   // Last just to avoid race conditions
            clear_decals
        )
        .run();
}

fn setup(
    mut commands: Commands, 
    mut window: Query<&mut Window>, 
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>, 
    mut sprays: ResMut<SprayMaterials>,
) {
    let mut window = window.single_mut();
    window.title = String::from("Minimal FPS Controller Example");
    // commands.spawn(Window { title: "Minimal FPS Controller Example".to_string(), ..default() });

    let colors = [
        tailwind::RED_700,
        tailwind::GREEN_600,
        tailwind::BLUE_700,
        tailwind::YELLOW_200,
        tailwind::PURPLE_300,
    ];

    let textures = [
        assets.load("splatter1.png"),
        assets.load("splatter2.png"),
        assets.load("splatter3.png"),
    ];

    for i in 0..colors.len() {
        sprays.0.push(
            standard_materials.add(
                StandardMaterial {
                    base_color: (colors[i % colors.len()] * 2.).into(),
                    base_color_texture: Some(textures[i % textures.len()].clone()),
                    // Preferably use mask for these if you can. Blend can create artifacts due to built in blend sorting
                    alpha_mode: AlphaMode::Mask(0.5),
                    perceptual_roughness: 1.,
                    ..default()
                }
            )
        )
    }

    sprays.0.push(
        standard_materials.add(
            StandardMaterial {
                base_color_texture: Some(assets.load("graffiti1.png").clone()),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }
        )
    );

    sprays.0.push(
        standard_materials.add(
            StandardMaterial {
                base_color_texture: Some(assets.load("graffiti2.png").clone()),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }
        )
    );

    sprays.0.push(
        standard_materials.add(
            StandardMaterial {
                base_color_texture: Some(assets.load("graffiti3.png").clone()),
                alpha_mode: AlphaMode::Mask(0.5),
                unlit: true,
                ..default()
            }
        )
    );

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(1.),
        SceneBundle {
            scene: assets
                .load(GltfAssetLabel::Scene(0).from_asset("sphere.glb")),
            transform: Transform::from_translation(Vec3::Y * 10.),
            ..default()
        }
    ));

    // Spawn some spheres!

    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(1.),
        SceneBundle {
            scene: assets
                .load(GltfAssetLabel::Scene(0).from_asset("sphere.glb")),
            transform: Transform::from_translation(Vec3::Y * 10.),
            ..default()
        }
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(1.),
        SceneBundle {
            scene: assets
                .load(GltfAssetLabel::Scene(0).from_asset("sphere.glb")),
            transform: Transform::from_translation(Vec3::Y * 10. + Vec3::X * 5.),
            ..default()
        }
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(1.),
        SceneBundle {
            scene: assets
                .load(GltfAssetLabel::Scene(0).from_asset("sphere.glb")),
            transform: Transform::from_translation(Vec3::Y * 10. + -Vec3::X * 5.).with_scale(Vec3::ONE * 3.),
            ..default()
        }
    ));

    // Note that we have two entities for the player
    // One is a "logical" player that handles the physics computation and collision
    // The other is a "render" player that is what is displayed to the user
    // This distinction is useful for later on if you want to add multiplayer,
    // where often time these two ideas are not exactly synced up
    let height = 3.0;
    let logical_entity = commands
        .spawn((
            Collider::cylinder(height / 2.0, 0.5),
            // A capsule can be used but is NOT recommended
            // If you use it, you have to make sure each segment point is
            // equidistant from the translation of the player transform
            // Collider::capsule_y(height / 2.0, 0.5),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            ActiveEvents::COLLISION_EVENTS,
            Velocity::zero(),
            RigidBody::Dynamic,
            Sleeping::disabled(),
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            GravityScale(0.0),
            Ccd { enabled: true }, // Prevent clipping when going fast
            TransformBundle::from_transform(Transform::from_translation(SPAWN_POINT)),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                yaw: TAU * 5.0 / 8.0,
                ..default()
            },
            FpsController {
                air_acceleration: 80.0,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .id();

    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 5.0,
                ..default()
            }),
            exposure: Exposure::SUNLIGHT,
            ..default()
        },
        RenderPlayer { logical_entity },
    ));


    commands.insert_resource(MainScene {
        handle: assets.load("playground.glb"),
        is_loaded: false,
    });

    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font: assets.load("fira_mono.ttf"),
                font_size: 24.0,
                color: Color::BLACK,
            },
        )
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            }),
    );
}

fn respawn(mut query: Query<(&mut Transform, &mut Velocity)>) {
    for (mut transform, mut velocity) in &mut query {
        if transform.translation.y > -50.0 {
            continue;
        }

        velocity.linvel = Vec3::ZERO;
        transform.translation = SPAWN_POINT;
    }
}

#[derive(Resource)]
struct MainScene {
    handle: Handle<Gltf>,
    is_loaded: bool,
}

fn scene_colliders(
    mut commands: Commands,
    mut main_scene: ResMut<MainScene>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_mesh_assets: Res<Assets<GltfMesh>>,
    gltf_node_assets: Res<Assets<GltfNode>>,
    mesh_assets: Res<Assets<Mesh>>,
) {
    if main_scene.is_loaded {
        return;
    }

    let gltf = gltf_assets.get(&main_scene.handle);

    if let Some(gltf) = gltf {
        let scene = gltf.scenes.first().unwrap().clone();
        commands.spawn(SceneBundle { scene, ..default() });
        for node in &gltf.nodes {
            let node = gltf_node_assets.get(node).unwrap();
            if let Some(gltf_mesh) = node.mesh.clone() {
                let gltf_mesh = gltf_mesh_assets.get(&gltf_mesh).unwrap();
                for mesh_primitive in &gltf_mesh.primitives {
                    let mesh = mesh_assets.get(&mesh_primitive.mesh).unwrap();
                    commands.spawn((
                        Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh).unwrap(),
                        RigidBody::Fixed,
                        TransformBundle::from_transform(node.transform),
                    ));
                }
            }
        }
        main_scene.is_loaded = true;
    }
}

#[derive(Resource, Default)]
pub struct SprayMaterials(Vec<Handle<StandardMaterial>>);

fn painter(
    mut commands: Commands,
    materials: Res<SprayMaterials>,
    btn: Res<ButtonInput<MouseButton>>,
    player: Query<&Transform, With<RenderPlayer>>,
    mut material_index: Local<usize>,
) {
    if btn.just_pressed(MouseButton::Left) {
        for transform in player.iter() {
            let spray_transform = transform.with_scale(Vec3::ONE * 2. + Vec3::Z * 50.);

            if materials.0.is_empty() {
                panic!("No materials to spray with!");
            }

            spray_decal(&mut commands, materials.0[*material_index % materials.0.len()].clone(), spray_transform);
            *material_index = (*material_index + 1) % materials.0.len();
        }
    }
}

fn manage_cursor(
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window>,
    mut controller_query: Query<&mut FpsController>,
) {
    for mut window in &mut window_query {
        if btn.just_pressed(MouseButton::Left) {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
            for mut controller in &mut controller_query {
                controller.enable_input = true;
            }
        }
        if key.just_pressed(KeyCode::Escape) {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
            for mut controller in &mut controller_query {
                controller.enable_input = false;
            }
        }
    }
}

fn display_text(
    mut controller_query: Query<(&Transform, &Velocity)>,
    mut text_query: Query<&mut Text>,
) {
    for (transform, velocity) in &mut controller_query {
        for mut text in &mut text_query {
            text.sections[0].value = format!(
                "vel: {:.2}, {:.2}, {:.2}\npos: {:.2}, {:.2}, {:.2}\nspd: {:.2}\nPress C to clear decals!\nIf an object has too many decals, decaling won't work!",
                velocity.linvel.x,
                velocity.linvel.y,
                velocity.linvel.z,
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                velocity.linvel.xz().length()
            );
        }
    }
}

fn make_all_decalable( // Make absolutely everything decalable, just for demonstration purposes
    mut commands: Commands,
    entities: Query<Entity, (With<Handle<Mesh>>, Without<Decal>, Without<Decalable>)>,
) {
    for entity in entities.iter() {
        commands.entity(entity).insert(Decalable::default());
    }
}

fn clear_decals( // Make absolutely everything decalable, just for demonstration purposes
    mut commands: Commands,
    key: Res<ButtonInput<KeyCode>>,
    decals: Query<Entity, With<Decal>>,
    decalables: Query<Entity, With<Decalable>>,
) {
    if key.just_pressed(KeyCode::KeyC) {
        for entity in decals.iter() {
            commands.entity(entity).despawn();
        }

        for entity in decalables.iter() {
            commands.entity(entity).insert(Decalable::default());
        }
    }
}