use bevy::pbr::NotShadowCaster;

use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;

pub mod prelude;

const DECAL_REMOVE_BACKFACES: bool = true; // When false, both sides of the mesh will be sprayed with a decal
const DECAL_MAX_PER_ENTTIY: usize = 16;    // Max number of decals you can stick on one entity
const DECAL_EPSILON: f32 = 0.00016;        // The offset of the decal from the base mesh, to prevent Z-fighting

/// Decalable component. Add this to entities that you wish to apply decals onto.
/// 
/// # Example:
/// 
/// ```
/// commands.entity(my_entity).insert(Decalable::default());
/// ```
#[derive(Component, Default)]
pub struct Decalable(usize); // Stores the number of decals already applied

/// # Example:
/// 
/// ```
/// spray_decal(
///     &mut commands,
///     // Handle to your material
///     my_material.clone(),
///     // Transform of the decal. Will apply towards transform.forward(), 
///     // in this case it's projecting directly down. Scale can be used
///     // to set the size and reach of the Decal.
///     Transform::from_translation(Vec3::ZERO)
///         .with_scale(Vec3::ONE * 2. + Vec3::Z * 10.)
///         .looking_to(Vec3::NEG_Y, Vec3::Y),
/// );
/// ```
/// 
/// # Note
/// 
/// The bounding box of the Decals transform must intersect
/// with the vertices of the model it's being applied to, in
/// world space. Decals will only be applied to entities
/// with the Decalable component. This function will try to
/// spray a decal only once after called.
pub fn spray_decal(commands: &mut Commands, material: Handle<StandardMaterial>, transform: Transform) {
    // This entity will be removed once the decals has been applied
    commands.spawn((
        transform,
        ApplyingDecal(material),
    ));
}

#[derive(Component)]
pub struct Decal;   // Marker component for all decals

pub struct DecalPlugin;

impl Plugin for DecalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, decal_system); 
    }
}

#[derive(Component)]
struct ApplyingDecal(Handle<StandardMaterial>);

#[derive(Clone, Copy)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
}

impl Vertex {
    pub fn lerp(&self, rhs: Vertex, d: f32) -> Vertex {
        return Vertex {
            position: self.position.lerp(rhs.position, d),
            normal: self.normal.lerp(rhs.normal, d),
            uv: self.uv.lerp(rhs.uv, d),
        }
    }
}

struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,
}

fn is_inside_unit_cube (p: Vec3) -> bool {
    return p.x.abs() <= 1. && p.y.abs() <= 1. && p.z.abs() <= 1.;
}

// Create a new triangle between a, ab, ac
fn new_triangle(
    a: Vertex, b: Vertex, c: Vertex,
    fa: f32, fb: f32, fc: f32,
    triangles: &mut Vec<Triangle>,
) {
    let d_ab = (1. - fa) / (fb - fa);
    let d_ac = (1. - fa) / (fc - fa);
    let ab = a.lerp(b, d_ab);
    let ac = a.lerp(c, d_ac);
    triangles.push(
        Triangle {
            a: a,
            b: ab,
            c: ac,
        }
    );
}

// Create two new triangles between b, c, ab, ac
fn new_quad(
    a: Vertex, b: Vertex, c: Vertex,
    fa: f32, fb: f32, fc: f32,
    triangles: &mut Vec<Triangle>,
) {
    let db = (1. - fa) / (fb - fa);
    let dc = (1. - fa) / (fc - fa);
    let ab = a.lerp(b, db);
    let ac = a.lerp(c, dc);

    triangles.push(
        Triangle {
            a: b,
            b: c,
            c: ac,
        }
    );
    triangles.push(
        Triangle {
            a: b,
            b: ac,
            c: ab,
        }
    );
}

// Attempt to slice the triangle along the plane defined by the axis-aligned normal
fn slice(
    triangle: &mut Triangle,
    normal: Vec3,
    triangles: &mut Vec<Triangle>,
) -> bool {
    let fa = triangle.a.position.dot(normal);
    let fb = triangle.b.position.dot(normal);
    let fc = triangle.c.position.dot(normal);

    if fa > 1. && fb > 1. && fc > 1. { // Triangle is outside of the projection volume
        return true;
    }

    if fa < 1. && fb > 1. && fc > 1. {
        new_triangle(triangle.a, triangle.b, triangle.c, fa, fb, fc, triangles);
        return true;
    }

    if fa > 1. && fb < 1. && fc > 1. {
        new_triangle(triangle.b, triangle.c, triangle.a, fb, fc, fa, triangles);
        return true;
    }

    if fa > 1. && fb > 1. && fc < 1. {
        new_triangle(triangle.c, triangle.a, triangle.b, fc, fa, fb, triangles);
        return true;
    }
    // Quads
    if fa > 1. && fb < 1. && fc < 1. {
        new_quad(triangle.a, triangle.b, triangle.c, fa, fb, fc, triangles);
        return true;
    }

    if fa < 1. && fb > 1. && fc < 1. {
        new_quad(triangle.b, triangle.c, triangle.a, fb, fc, fa, triangles);
        return true;
    }

    if fa < 1. && fb < 1. && fc > 1. {
        new_quad(triangle.c, triangle.a, triangle.b, fc, fa, fb, triangles);
        return true;
    }

    return false;
}

fn apply_decal(
    mesh: &Mesh,
    mesh_transform: &Transform,
    decal_transform: &Transform,
    offset: f32,
) -> Option<Mesh> {
    let vertex_attribute = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let normal_attribute = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap();
    let indices = mesh.indices().unwrap();

    let VertexAttributeValues::Float32x3(vertex_attribute) = vertex_attribute else {
        panic!("Unexpected vertex format, expected Float32x3.");
    };

    let VertexAttributeValues::Float32x3(normal_attribute) = normal_attribute else {
        panic!("Unexpected normal format, expected Float32x3.");
    };

    let Indices::U16(indices) = indices else {
        panic!("Unexpected indices format, expected U16.");
    };
    
    let mut axii = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::NEG_X,
        Vec3::NEG_Y,
        Vec3::NEG_Z,
    ];

    let decal_proj = decal_transform.compute_matrix().inverse();
    let inv_decal_transform = Transform::from_matrix(decal_proj);

    let mut new_triangles = Vec::with_capacity(1024);

    for triangle in indices.chunks(3) {
        let vA = Vec3::from(vertex_attribute[triangle[0] as usize]) + Vec3::from(normal_attribute[triangle[0] as usize]) * offset;
        let vB = Vec3::from(vertex_attribute[triangle[1] as usize]) + Vec3::from(normal_attribute[triangle[1] as usize]) * offset;
        let vC = Vec3::from(vertex_attribute[triangle[2] as usize]) + Vec3::from(normal_attribute[triangle[2] as usize]) * offset;

        let pA = decal_proj.transform_point3(mesh_transform.transform_point(vA));
        let pB = decal_proj.transform_point3(mesh_transform.transform_point(vB));
        let pC = decal_proj.transform_point3(mesh_transform.transform_point(vC));
        

        let mut removed = false;
        for axis in axii.iter() {
            let fA = pA.dot(*axis);
            let fB = pB.dot(*axis);
            let fC = pC.dot(*axis);

            if fA > 1. && fB > 1. && fC > 1. {
                removed = true;
                break;
            }
        }
        if removed {
            continue;
        }

        let nA = inv_decal_transform.rotation * (mesh_transform.rotation * Vec3::from(normal_attribute[triangle[0] as usize]));
        let nB = inv_decal_transform.rotation * (mesh_transform.rotation * Vec3::from(normal_attribute[triangle[1] as usize]));
        let nC = inv_decal_transform.rotation * (mesh_transform.rotation * Vec3::from(normal_attribute[triangle[2] as usize])); 

        // Set this to false to apply the decal to both sides of the mesh.

        if DECAL_REMOVE_BACKFACES {
            let normal = nA + nB + nC;
            if normal.z < 0. {
                continue;
            }
        }


        let A = Vertex { position: pA, normal: nA, uv: Vec2::ZERO };
        let B = Vertex { position: pB, normal: nB, uv: Vec2::ZERO };
        let C = Vertex { position: pC, normal: nC, uv: Vec2::ZERO };

        if is_inside_unit_cube(A.position) && is_inside_unit_cube(B.position) && is_inside_unit_cube(C.position) {
            new_triangles.push(Triangle {a: A, b: B, c: C});
            continue;
        }

        let mut input_triangles = Vec::with_capacity(1024);
        let mut output_triangles = Vec::with_capacity(1024);
        input_triangles.push(Triangle {a: A, b: B, c: C});

        for axis in axii.iter() {
            while input_triangles.len() > 0 {
                let mut triangle = input_triangles.pop().unwrap();
                if !slice(&mut triangle, *axis, &mut output_triangles) {
                    output_triangles.push(triangle);
                }
            }
            if axis != axii.last().unwrap() {
                let tmp = input_triangles;
                input_triangles = output_triangles;
                output_triangles = tmp;
            }
        }

        while output_triangles.len() > 0 {
            new_triangles.push(output_triangles.pop().unwrap());
        }
  
    }

    let mut positions = Vec::with_capacity(4096);
    let mut normals = Vec::with_capacity(4096);
    let mut uvs = Vec::with_capacity(4096);
    let mut indices = Vec::with_capacity(4096);
    let mut index: u16 = 0;

    for triangle in new_triangles.iter() {
        positions.push(triangle.a.position);
        positions.push(triangle.b.position);
        positions.push(triangle.c.position);
        normals.push(triangle.a.normal);
        normals.push(triangle.b.normal);
        normals.push(triangle.c.normal);
        indices.push(index);
        index += 1;
        indices.push(index);
        index += 1;
        indices.push(index);
        index += 1;
    }

    if positions.len() == 0 {
        return None
    }

    for i in 0..positions.len() {
        uvs.push(Vec2::new(positions[i].x*0.5+0.5, positions[i].y*0.5+0.5));
    }

    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            positions
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            uvs,
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            normals,
        )
        .with_inserted_indices(Indices::U16(indices));
    return Some(mesh)
}


fn decal_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut decals: Query<(Entity, &Transform, &ApplyingDecal)>, 
    mut models: Query<(Entity, &Handle<Mesh>, &Transform, &GlobalTransform, &mut Decalable)>,
) {
    for (decal_entity, transform,  decal) in decals.iter_mut() {
        for (model_entity, model_mesh, model_transform, global_transform, mut decalable) in models.iter_mut() {
            if decalable.0 >= DECAL_MAX_PER_ENTTIY {
                continue;
            }

            let mesh_transform = Transform::from(global_transform.mul_transform(*model_transform));

            if let Some(mesh) = apply_decal(meshes.get(model_mesh).unwrap(), &mesh_transform, transform, (decalable.0 + 1) as f32 * DECAL_EPSILON) {

                let applied_decal = commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh).clone(),
                        material: decal.0.clone(),
                        transform: Transform::from_matrix(mesh_transform.compute_matrix().inverse()).mul_transform(*transform), // Inverse stuff to make it work with Bevy's transform propagation
                        ..default()
                    },
                    NotShadowCaster,    // For extra performance
                    Decal,
                )).id();

                commands.entity(model_entity).add_child(applied_decal);
                decalable.0 += 1;
            }
        }

        commands.entity(decal_entity).despawn();
    }

}
