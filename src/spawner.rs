use bevy::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::SimulationParams;

pub struct SpawnerPlugin;

impl Plugin for SpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_particles);
        app.add_systems(Update, adjust_particle_count);
    }
}

fn spawn_particles(
    mut commands: Commands,
    params: Res<SimulationParams>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_n_particles(&mut commands, &params, &mut meshes, &mut materials, params.particle_count);
}

pub fn adjust_particle_count(
    mut commands: Commands,
    params: Res<SimulationParams>,
    query: Query<Entity, With<Particle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let current_count = query.iter().count();
    if current_count < params.particle_count {
        spawn_n_particles(
            &mut commands,
            &params,
            &mut meshes,
            &mut materials,
            params.particle_count - current_count,
        );
    } else if current_count > params.particle_count {
        let to_remove = current_count - params.particle_count;
        for entity in query.iter().take(to_remove) {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_n_particles(
    commands: &mut Commands,
    params: &SimulationParams,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    count: usize,
) {
    let mut rng = rand::thread_rng();

    let circle_mesh = meshes.add(Circle::new(1.0));

    let range_x = params.region_size.x * 0.5;
    let range_y = params.region_size.y * 0.5;

    let default_scale = params.scale;

    for _ in 0..count {
        let p_type = rng.gen_range(0..params.particle_types);
        let color = params.colors[p_type];
        
        let px = rng.gen_range(-range_x..range_x);
        let py = rng.gen_range(-range_y..range_y);
        
        // Random point inside unit circle for initial velocity jitter
        let theta = rng.gen_range(0.0..std::f32::consts::TAU);
        let r = rng.gen_range(0.0f32..1.0).sqrt();
        let vx = r * theta.cos() * 0.1;
        let vy = r * theta.sin() * 0.1;

        commands.spawn((
            Particle,
            Velocity(Vec2::new(vx, vy)),
            ParticleType(p_type),
            Mesh2d(circle_mesh.clone()),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(px, py, 0.0)
                .with_scale(Vec3::splat(default_scale)),
        ));
    }
}
