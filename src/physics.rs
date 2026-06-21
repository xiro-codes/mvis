use bevy::prelude::*;

use crate::components::*;
use crate::SimulationParams;
use crate::spatial_hash::{SpatialHashGrid, DensityGrid};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, simulate_particles);
    }
}

#[derive(Clone, Copy)]
struct ParticleData {
    entity: Entity,
    position: Vec2,
    kind: ParticleType,
}

fn simulate_particles(
    params: Res<SimulationParams>,
    mut local_grid: Local<Option<SpatialHashGrid>>,
    mut local_density: Local<Option<DensityGrid>>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &ParticleType,
    )>,
) {
    let time_scale = params.time_scale;
    let min_dist = params.min_dist;
    let interaction_radius = params.interaction_radius;
    
    if local_grid.is_none() {
        *local_grid = Some(SpatialHashGrid::new(interaction_radius));
    }
    if local_density.is_none() {
        // Use a cell size proportional to bounds or simply a fixed value like 150.0 for density
        let density_cell_size = 150.0;
        *local_density = Some(DensityGrid::new(density_cell_size));
    }

    let grid = local_grid.as_mut().unwrap();
    let density_grid = local_density.as_mut().unwrap();

    grid.clear();
    density_grid.clear();

    // 1. Build snapshot and grids
    let snapshot: Vec<ParticleData> = query
        .iter()
        .map(|(e, t, _, pk)| {
            ParticleData {
                entity: e,
                position: t.translation.truncate(),
                kind: *pk,
            }
        })
        .collect();

    for (i, data) in snapshot.iter().enumerate() {
        grid.insert(i, data.position);
        // Splat density into grid for all particles to calculate local density score
        density_grid.add_density(data.position, 1.0);
    }

    let interaction_matrix = params.interaction_matrix;
    let density_limit = params.density_limit;
    let dampening = params.dampening;
    let force_multiplier = params.attraction_strength;

    query.par_iter_mut().for_each(|(entity, mut transform, mut velocity, pk)| {
        let mut position = transform.translation.truncate();
        let mut vel = velocity.0;
        let mut total_force = Vec2::ZERO;

        let p_kind = *pk;

        // Evaluate local density score for density regulation
        let local_density = density_grid.evaluate(position);
        let mut density_factor = 1.0;
        if local_density > density_limit {
            // Exactly matching reference: 1.0f - min(max(0.0f, local_density - density_limit), 1.005f)
            density_factor = 1.0 - (local_density - density_limit).max(0.0).min(1.005);
        }

        // Search neighbors in O(1) grid cells instead of O(N) loop
        for &other_idx in grid.get_neighbors(position) {
            let other = &snapshot[other_idx];
            if entity == other.entity {
                continue;
            }

            let dir = other.position - position;
            
            let dist_sqr = dir.length_squared().max(0.01);
            
            // Only interact if within our spatial grid radius
            if dist_sqr > interaction_radius * interaction_radius {
                continue;
            }
            
            let dist = dist_sqr.sqrt();
            let dir_norm = dir / dist;

            // Particle Life Force (Interaction Matrix)
            let mut attraction = interaction_matrix[p_kind.index()][other.kind.index()];
            
            // Density Regulation
            if attraction > 0.0 {
                attraction *= density_factor;
            }

            let force = if dist < min_dist {
                // Repulsion zone
                (dist / min_dist - 1.0) * 2.0
            } else {
                // Interaction zone
                let width = interaction_radius - min_dist;
                let midpoint = (min_dist + interaction_radius) * 0.5;
                attraction * (1.0 - (dist - midpoint).abs() / (width * 0.5))
            };

            // dir_norm points from `position` to `other.position` (a to b)
            // A positive force here means pulling `a` towards `b`. 
            // A negative force (like repulsion above where dist < min_dist) points `a` away from `b`.
            total_force += dir_norm * force * force_multiplier;
        }

        // Cap force at MAX_FORCE
        let max_force = 100.0;
        if total_force.length_squared() > max_force * max_force {
            total_force = total_force.normalize() * max_force;
        }

        vel += total_force * time_scale;
        vel *= dampening;
        
        position += vel * time_scale;

        transform.translation.x = position.x;
        transform.translation.y = position.y;
        velocity.0 = vel;
    });
}
