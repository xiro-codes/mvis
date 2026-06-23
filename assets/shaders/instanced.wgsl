#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    kind: u32,
    padding: u32,
};

struct SimParams {
    interaction_matrix: array<vec4<f32>, 25>, // 100 floats packed into 25 vec4s to avoid alignment issues
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
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
};

@group(1) @binding(0) var<uniform> params: SimParams;
@group(1) @binding(1) var<storage, read> particles: array<Particle>;

const QUAD_VERTS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5, -0.5),
    vec2<f32>(-0.5,  0.5),
    vec2<f32>(-0.5,  0.5),
    vec2<f32>( 0.5, -0.5),
    vec2<f32>( 0.5,  0.5),
);

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var out: VertexOutput;
    
    let vert = QUAD_VERTS[vertex_index];
    let uv = vert * 2.0; // range -1 to 1 for dist sq calc
    
    let p = particles[instance_index];
    let color = params.colors[p.kind];
    
    let scale = params.scale;
    let world_position = vec3<f32>(vert * scale, 0.0) + vec3<f32>(p.position, 0.0);
    
    out.clip_position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.color = color;
    out.uv = uv; // pass through the local coordinates -1 to 1
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Render a soft circle
    let dist_sq = dot(in.uv, in.uv);
    if (dist_sq > 1.0) {
        discard;
    }
    
    // Optional: add a slight glow/soft edge
    let alpha = smoothstep(1.0, 0.8, sqrt(dist_sq));
    let intensity = params.emission_intensity; // User-controlled HDR boost
    // Pre-multiply RGB with alpha so additive blending correctly fades the edges
    return vec4<f32>(in.color.rgb * in.color.a * alpha * intensity, in.color.a * alpha);
}
