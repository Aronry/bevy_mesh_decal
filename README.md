# bevy_mesh_decal

Fast, optimal, real-time decal spraying. Based on [Unity MeshDecal](https://github.com/Fewes/MeshDecal).

# Examples

Dynamic physics objects and complex meshes: [`examples/paint_thrower.rs`](./examples/paint_thrower.rs). Try 
it out with `cargo run --example paint_thrower`.

![2025-05-10 21-02-34](https://github.com/user-attachments/assets/9bd3dbb2-a576-4a11-bf82-51dd8d9cde51)


# Usage

Check out the [examples](./examples) for details. Tl;dr initialize the plugin with
```rust
app.add_plugin(DecalPlugin)
```
and spawn sprites with
```rust
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // Material of the spray
    let spray_material = standard_materials.add(  
        StandardMaterial {
            base_color_texture: Some(assets.load("graffiti1.png").clone()),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        }
    )

    // Spawn an object to spray a Decal on
    commands.spawn((
        PbrBundle {
            mesh: assets.load("cube.obj"),
            material: standard_materials.add(
                StandardMaterial {
                    base_color: Color::GREEN,
                    ..default()
                }
            ),
            transform: Transform::from_translation(Vec3::NEG_Y * 5.),
            ..default()
        }
    ));

    // Spray transform. In this case spraying straight down, scaled by 2 and reaching 12 meters down
    let spray_transform = Transform::from_translation(Vec3::Y)  
        .with_scale(Vec3::ONE * 2. + Vec3::Z * 10.)
        .looking_to(Vec3::NEG_Y, Vec3::Y);

    // Finally spray the decal, which will be applied next frame
    spray_decal(&mut commands, spray_material.clone(), spray_transform);
}
```


## Versioning

| `bevy_sprite3d` version | `bevy` version |
|-------------------------|----------------|
| 1.0.0                   | 0.14           |
