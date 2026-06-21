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

    // Global gravity towards origin
    let dist_to_center = length(p.position);
    if (dist_to_center > 0.0) {
        let dir_to_center = -p.position / dist_to_center;
        total_force += dir_to_center * (dist_to_center * params.global_gravity); 
    }

    p.velocity += total_force * params.time_scale;
    p.velocity *= params.dampening;
    p.position += p.velocity * params.time_scale;

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
