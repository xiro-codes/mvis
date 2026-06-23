struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    kind: u32,
    padding: u32,
};

struct SimParams {
    interaction_matrix: array<vec4<f32>, 25>, // 10x10 flattened packed into vec4s
    colors: array<vec4<f32>, 10>,
    attraction_strength: f32,
    time_scale: f32,
    min_dist: f32,
    interaction_radius: f32,
    density_limit: f32,
    dampening: f32,
    particle_count: u32,
    region_size_x: f32,
    region_size_y: f32,
    scale: f32,
    global_gravity: f32,
    infinite_space: u32,
    emission_intensity: f32,
    gravity_wells: u32,
    gravity_well_radius: f32,
    gravity_center_well: u32,
    gravity_well_pattern: u32,
    gravity_well_rotation: f32,
    mouse_pos: vec2<f32>,
    gravity_well_distance_power: f32,
    record_exclusion_zone: u32,
    record_radius: f32,
    spawn_seed: u32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var p = particles[index];
    var total_force = vec2<f32>(0.0, 0.0);
    
    // Evaluate density (simple O(N) approach)
    var local_density = 0.0;
    let interaction_radius_sq = params.interaction_radius * params.interaction_radius;
    
    for (var i: u32 = 0u; i < params.particle_count; i = i + 1u) {
        if (i == index) { continue; }
        let other = particles[i];
        let dir = other.position - p.position;
        let dist_sq = dot(dir, dir);
        if (dist_sq < interaction_radius_sq) {
            // Rough density approximation based on nearby particles
            local_density += 1.0; 
        }
    }
    
    var density_factor = 1.0;
    if (local_density > params.density_limit) {
        density_factor = 1.0 - clamp(local_density - params.density_limit, 0.0, 1.005);
    }

    for (var i: u32 = 0u; i < params.particle_count; i = i + 1u) {
        if (i == index) { continue; }
        let other = particles[i];
        
        let dir = other.position - p.position;
        let dist_sq = max(dot(dir, dir), 0.01);
        
        if (dist_sq > interaction_radius_sq) {
            continue;
        }
        
        let dist = sqrt(dist_sq);
        let dir_norm = dir / dist;
        
        let mat_idx = p.kind * 10u + other.kind;
        let vec_idx = mat_idx / 4u;
        let comp_idx = mat_idx % 4u;
        var attraction = params.interaction_matrix[vec_idx][comp_idx];
        
        if (attraction > 0.0) {
            attraction *= density_factor;
        }
        
        var force = 0.0;
        if (dist < params.min_dist) {
            // Repulsion zone
            let r = dist / params.min_dist;
            force = (r - 1.0) * 3.0; // Stronger, clean repulsion
        } else {
            // Interaction zone
            let width = params.interaction_radius - params.min_dist;
            let t = (dist - params.min_dist) / width;
            let PI = 3.14159265359;
            // Smooth sine bump from 0 up to 1 and down to 0
            force = attraction * sin(t * PI);
        }
        
        total_force += dir_norm * force * params.attraction_strength;
    }

    // Cap force
    let max_force = 100.0;
    if (dot(total_force, total_force) > max_force * max_force) {
        total_force = normalize(total_force) * max_force;
    }

    // Global gravity towards configurable centers
    var well_force = vec2<f32>(0.0, 0.0);
    
    // Center well has fixed weight of 1.0 for all particles
    if (params.gravity_center_well > 0u) {
        let p_to_center = params.mouse_pos - p.position;
        let d = length(p_to_center);
        if (d > 0.0) {
            let dir_to_center = p_to_center / d;
            let force_mag = (params.global_gravity * 100000.0) / (d + 50.0);
            well_force += dir_to_center * force_mag;
        }
    }

    let num_wells = params.gravity_wells;
    if (num_wells > 0u && params.gravity_well_pattern != 0u) {
        for (var i = 0u; i < num_wells; i++) {
            var well_pos = vec2<f32>(0.0, 0.0);
            
            if (params.gravity_well_pattern == 1u) {
                // Ring
                let angle = (f32(i) / f32(num_wells)) * 6.28318530718; // TAU
                well_pos = vec2<f32>(cos(angle), sin(angle)) * params.gravity_well_radius;
            } else if (params.gravity_well_pattern == 2u) {
                // Grid
                let cols = max(1u, u32(ceil(sqrt(f32(num_wells)))));
                let row = i / cols;
                let col = i % cols;
                let rows = max(1u, (num_wells + cols - 1u) / cols);
                let offset_x = (f32(col) - f32(cols - 1u) * 0.5) * params.gravity_well_radius;
                let offset_y = (f32(row) - f32(rows - 1u) * 0.5) * params.gravity_well_radius;
                well_pos = vec2<f32>(offset_x, offset_y);
            } else if (params.gravity_well_pattern == 3u) {
                // Horizontal Line
                let offset_x = (f32(i) - f32(num_wells - 1u) * 0.5) * params.gravity_well_radius;
                well_pos = vec2<f32>(offset_x, 0.0);
            } else if (params.gravity_well_pattern == 4u) {
                // Spiral
                let angle = f32(i) * 3.14159265359 * 1.5;
                let r = (f32(i) + 1.0) * (params.gravity_well_radius * 0.2);
                well_pos = vec2<f32>(cos(angle), sin(angle)) * r;
            } else if (params.gravity_well_pattern == 5u) {
                // Star
                let angle = (f32(i) / f32(num_wells)) * 6.28318530718;
                var r = params.gravity_well_radius;
                if (i % 2u != 0u) {
                    r = params.gravity_well_radius * 0.4;
                }
                well_pos = vec2<f32>(cos(angle), sin(angle)) * r;
            } else if (params.gravity_well_pattern == 6u) {
                // Cross
                let arms = 4u;
                let points_per_arm = (num_wells + arms - 1u) / arms;
                let arm_idx = i % arms;
                let point_idx = i / arms;
                
                let angle = (f32(arm_idx) / f32(arms)) * 6.28318530718;
                let r = ((f32(point_idx) + 1.0) / f32(points_per_arm)) * params.gravity_well_radius;
                well_pos = vec2<f32>(cos(angle), sin(angle)) * r;
            } else if (params.gravity_well_pattern == 7u) {
                // Random
                let hash1 = (i * 374761393u) + params.spawn_seed;
                let hash2 = (i * 668265263u) + params.spawn_seed;
                let rand_angle = (f32(hash1) / 4294967295.0) * 6.28318530718;
                let rand_r = (f32(hash2) / 4294967295.0) * params.gravity_well_radius;
                well_pos = vec2<f32>(cos(rand_angle), sin(rand_angle)) * rand_r;
            }
            
            let rot_cos = cos(params.gravity_well_rotation);
            let rot_sin = sin(params.gravity_well_rotation);
            let rx = well_pos.x * rot_cos - well_pos.y * rot_sin;
            let ry = well_pos.x * rot_sin + well_pos.y * rot_cos;
            well_pos = vec2<f32>(rx, ry) + params.mouse_pos;
            
            let d = distance(p.position, well_pos);
            
            if (d > 0.0) {
                let dist_from_center = distance(params.mouse_pos, well_pos);
                let power_mult = 1.0 + (dist_from_center * 0.01) * params.gravity_well_distance_power;
                
                let dir_to_center = (well_pos - p.position) / d;
                let force_mag = ((params.global_gravity * power_mult) * 100000.0) / (d + 50.0);
                well_force += dir_to_center * force_mag;
            }
        }
    }
    total_force += well_force;
    
    p.velocity += total_force * params.time_scale;
    p.velocity *= pow(params.dampening, abs(params.time_scale));
    p.position += p.velocity * params.time_scale;

    // Record Exclusion Zone logic (Hard Clamp & Bounce)
    if (params.record_exclusion_zone > 0u) {
        let dist_to_record = distance(p.position, params.mouse_pos);
        if (dist_to_record < params.record_radius) {
            // Use previous position to determine which side it entered from to prevent tunneling
            let prev_pos = p.position - p.velocity * params.time_scale;
            var dir_out = normalize(prev_pos - params.mouse_pos);
            
            // Fallback if prev_pos is exactly at mouse_pos
            if (length(prev_pos - params.mouse_pos) < 0.001) {
                dir_out = vec2<f32>(1.0, 0.0);
            }
            
            // Push particle strictly outside the record
            p.position = params.mouse_pos + dir_out * params.record_radius;
            
            // Bounce velocity off the record's edge (with slight damping)
            let dot_vel = dot(p.velocity, dir_out);
            if (dot_vel < 0.0) {
                p.velocity = p.velocity - 1.5 * dot_vel * dir_out;
            }
        }
    }

    // Wrap around boundaries
    if (params.infinite_space == 0u) {
        let rx = params.region_size_x * 0.5;
        let ry = params.region_size_y * 0.5;
        
        if (p.position.x > rx) { p.position.x -= params.region_size_x; }
        if (p.position.x < -rx) { p.position.x += params.region_size_x; }
        if (p.position.y > ry) { p.position.y -= params.region_size_y; }
        if (p.position.y < -ry) { p.position.y += params.region_size_y; }
    }

    particles[index] = p;
}
