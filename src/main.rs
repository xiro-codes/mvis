use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

mod components;
// mod physics; // removed
// mod spawner; // removed
// mod spatial_hash; // removed
mod gpu_pipeline;
mod instanced_render;

#[derive(Resource, Clone, ExtractResource)]
pub struct SimulationParams {
    pub particle_count: usize,
    pub particle_types: usize,
    pub attraction_strength: f32,
    pub time_scale: f32,
    pub min_dist: f32,
    pub region_size: Vec2,
    pub scale: f32,
    pub interaction_matrix: [[f32; 10]; 10],
    pub colors: [Color; 10],
    pub density_limit: f32,
    pub interaction_radius: f32,
    pub dampening: f32,
    pub infinite_space: bool,
    pub global_gravity: f32,
    pub type_proportions: [f32; 10],
    pub spawn_seed: u32,
}

impl Default for SimulationParams {
    fn default() -> Self {
        // Particle Life interaction matrix up to 10 types.
        let mut interaction_matrix = [[0.0; 10]; 10];
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for i in 0..10 {
            for j in 0..10 {
                interaction_matrix[i][j] = rng.gen_range(-1.0..1.0);
            }
        }

        let colors = [
            Color::srgba(0.0, 1.0, 1.0, 1.0), // 1
            Color::srgba(1.0, 0.0, 0.0, 1.0), // 2
            Color::srgba(0.0, 1.0, 0.0, 1.0), // 3
            Color::srgba(1.0, 0.0, 1.0, 1.0), // 4
            Color::srgba(1.0, 1.0, 0.0, 1.0), // 5
            Color::srgba(0.0, 0.0, 1.0, 1.0), // 6
            Color::srgba(1.0, 0.5, 0.0, 1.0), // 7
            Color::srgba(0.5, 0.0, 1.0, 1.0), // 8
            Color::srgba(0.0, 1.0, 0.5, 1.0), // 9
            Color::srgba(1.0, 1.0, 1.0, 1.0), // 10
        ];

        Self {
            particle_count: 50_000, // Cranked up to 50k by default
            particle_types: 6,
            attraction_strength: 50.0,
            time_scale: 1.0,
            min_dist: 20.0,
            region_size: Vec2::new(2560.0, 1440.0),
            scale: 2.0, // Scale down visual size for high count
            interaction_matrix,
            colors,
            density_limit: 0.5,
            interaction_radius: 200.0,
            dampening: 0.9,
            infinite_space: false,
            global_gravity: 0.0,
            type_proportions: [1.0; 10],
            spawn_seed: 0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Universe Simulator".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<SimulationParams>()
        .add_plugins((
            gpu_pipeline::GpuPhysicsPlugin,
            instanced_render::InstancedRenderPlugin,
            EguiPlugin::default()
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(Update, (camera_movement, animate_time_scale))
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn ui_system(mut contexts: EguiContexts, mut params: ResMut<SimulationParams>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        egui::Window::new("Simulation Controls").show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut params.particle_count, 10..=200_000).text("Particle Count"));
            ui.add(egui::Slider::new(&mut params.particle_types, 1..=10).text("Particle Types"));
            ui.add(
                egui::Slider::new(&mut params.attraction_strength, 0.0..=200.0)
                    .text("Force Multiplier"),
            );
            ui.add(egui::Slider::new(&mut params.time_scale, 0.0..=2.0).text("Time Scale"));
            ui.add(egui::Slider::new(&mut params.dampening, 0.5..=1.0).text("Dampening"));
            ui.add(egui::Slider::new(&mut params.min_dist, 0.0..=200.0).text("Repulsion Radius"));
            ui.add(egui::Slider::new(&mut params.interaction_radius, 50.0..=500.0).text("Interaction Radius"));
            ui.add(egui::Slider::new(&mut params.density_limit, 0.0..=10.0).text("Density Limit"));
            ui.add(egui::Slider::new(&mut params.global_gravity, 0.0..=0.01).text("Global Gravity"));
            ui.checkbox(&mut params.infinite_space, "Infinite Space (No Bounds)");
            
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Randomize Rules").clicked() {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    for i in 0..10 {
                        for j in 0..10 {
                            params.interaction_matrix[i][j] = rng.gen_range(-1.0..1.0);
                        }
                    }
                }
                
                if ui.button("Randomize Proportions").clicked() {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    for i in 0..10 {
                        params.type_proportions[i] = rng.gen_range(0.1..2.0);
                    }
                    params.spawn_seed = params.spawn_seed.wrapping_add(1);
                }
                
                if ui.button("Randomize World").clicked() {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    params.attraction_strength = rng.gen_range(10.0..150.0);
                    params.min_dist = rng.gen_range(5.0..100.0);
                    params.interaction_radius = rng.gen_range(50.0..300.0);
                    params.density_limit = rng.gen_range(0.1..5.0);
                    params.dampening = rng.gen_range(0.8..0.99);
                    params.global_gravity = rng.gen_range(0.0..0.005);
                }
            });
            
            ui.separator();
            
            ui.collapsing("Type Proportions", |ui| {
                for i in 0..params.particle_types as usize {
                    ui.add(egui::Slider::new(&mut params.type_proportions[i], 0.0..=5.0).text(format!("Type {}", i)));
                }
                if ui.button("Apply Proportions (Respawn)").clicked() {
                    params.spawn_seed = params.spawn_seed.wrapping_add(1);
                }
            });

            ui.collapsing("Interaction Matrix", |ui| {
                egui::Grid::new("interaction_matrix_grid").show(ui, |ui| {
                    ui.label("");
                    for j in 0..params.particle_types as usize {
                        ui.label(format!("T{}", j));
                    }
                    ui.end_row();
                    
                    for i in 0..params.particle_types as usize {
                        ui.label(format!("Type {}", i));
                        for j in 0..params.particle_types as usize {
                            ui.add(
                                egui::DragValue::new(&mut params.interaction_matrix[i][j])
                                    .speed(0.01)
                                    .range(-2.0..=2.0)
                            );
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
}


fn setup_camera(mut commands: Commands) {
    // We adjust the camera scale so we can see the bounds.
    // With bounds at 10000, we need a massive scale to see everything
    commands.spawn((
        Camera2d,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        bevy::render::view::Hdr,
        bevy::post_process::bloom::Bloom::default(),
        bevy::render::view::Msaa::Off,
        Transform::from_scale(Vec3::splat(30.0))
    ));
}

fn camera_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let Ok(mut transform) = query.single_mut() else { return; };
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }
    
    // Scale movement speed by current zoom level so panning feels consistent
    let speed = 200.0 * transform.scale.x; 
    transform.translation += direction * speed * time.delta_secs();

    let mut zoom_delta = 0.0;
    if keyboard_input.pressed(KeyCode::KeyQ) {
        // Zoom out
        zoom_delta += 1.5 * time.delta_secs();
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        // Zoom in
        zoom_delta -= 1.5 * time.delta_secs();
    }
    
    if zoom_delta != 0.0 {
        transform.scale *= 1.0 + zoom_delta;
    }
}

fn animate_time_scale(
    mut params: ResMut<SimulationParams>,
    time: Res<Time>,
    mut last_seed: Local<u32>,
    mut last_count: Local<usize>,
) {
    let seed_changed = *last_seed != params.spawn_seed;
    let count_changed = *last_count != params.particle_count;
    
    // Re-initialize local tracking if they are 0 (first frame)
    if *last_seed == 0 && *last_count == 0 {
        *last_seed = params.spawn_seed;
        *last_count = params.particle_count;
        params.time_scale = 2.0;
    } else if seed_changed || count_changed {
        *last_seed = params.spawn_seed;
        *last_count = params.particle_count;
        params.time_scale = 2.0;
    }
    
    // Smoothly decay towards 0.05
    if params.time_scale > 0.05 {
        // use an exponential decay for a smoother slow-down
        params.time_scale -= (params.time_scale * 0.5) * time.delta_secs();
        if params.time_scale < 0.05 {
            params.time_scale = 0.05;
        }
    }
}
