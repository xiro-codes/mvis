use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use clap::Parser;

mod components;
// mod physics; // removed
// mod spawner; // removed
// mod spatial_hash; // removed
mod gpu_pipeline;
mod instanced_render;
mod audio_analysis;
mod mpd_client;
mod config;

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AnimateSource {
    Off,
    Sine,
    LowFreq,
    MidFreq,
    HighFreq,
}

#[derive(Resource)]
pub struct MpdState {
    pub receiver: crossbeam_channel::Receiver<(mpd_client::SongInfo, Option<Vec<u8>>)>,
    pub current_song: Option<mpd_client::SongInfo>,
    pub album_art: Option<Handle<Image>>,
}

#[derive(Component)]
struct MpdAlbumArtNode;

#[derive(Component)]
struct MpdTextNode;

fn animate_selector(ui: &mut egui::Ui, source: &mut AnimateSource) {
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(match *source {
            AnimateSource::Off => "Off",
            AnimateSource::Sine => "Sine",
            AnimateSource::LowFreq => "Bass",
            AnimateSource::MidFreq => "Mids",
            AnimateSource::HighFreq => "Treble",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(source, AnimateSource::Off, "Off");
            ui.selectable_value(source, AnimateSource::Sine, "Sine");
            ui.selectable_value(source, AnimateSource::LowFreq, "Bass");
            ui.selectable_value(source, AnimateSource::MidFreq, "Mids");
            ui.selectable_value(source, AnimateSource::HighFreq, "Treble");
        });
}

#[derive(Resource, Clone, ExtractResource, Serialize, Deserialize)]
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
    pub continuous_mutation: bool,
    pub is_animating_time: bool,
    pub target_time_scale: f32,
    pub animate_attraction: AnimateSource,
    pub animate_min_dist: AnimateSource,
    pub animate_interaction_radius: AnimateSource,
    pub animate_density_limit: AnimateSource,
    pub animate_dampening: AnimateSource,
    pub animate_global_gravity: AnimateSource,
    pub slider_animation_speed: f32,
    pub slider_animation_time: f32,
    pub audio_reactivity_power: f32,
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
            time_scale: 0.05,
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
            continuous_mutation: false,
            is_animating_time: true,
            target_time_scale: 0.05,
            animate_attraction: AnimateSource::Off,
            animate_min_dist: AnimateSource::Off,
            animate_interaction_radius: AnimateSource::Off,
            animate_density_limit: AnimateSource::Off,
            animate_dampening: AnimateSource::Off,
            animate_global_gravity: AnimateSource::Off,
            slider_animation_speed: 1.0,
            slider_animation_time: 0.0,
            audio_reactivity_power: 1.0,
        }
    }
}

fn main() {
    let app_config = config::AppConfig::load_or_create();
    let sim_params = app_config.simulation.clone();
    let mpd_config = app_config.mpd.clone();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Universe Simulator".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(sim_params)
        .insert_resource(mpd_config)
        .add_plugins((
            gpu_pipeline::GpuPhysicsPlugin,
            instanced_render::InstancedRenderPlugin,
            EguiPlugin::default()
        ))
        .add_systems(Startup, (setup_camera, setup_audio))
        .add_systems(Update, (camera_movement, update_audio_stream, update_mpd_state, animate_time_scale))
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn ui_system(mut contexts: EguiContexts, mut params: ResMut<SimulationParams>, mpd_config: Res<config::MpdConfig>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        egui::Window::new("Simulation Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Save Config").clicked() {
                    let mut app_config = config::AppConfig::default();
                    app_config.simulation = params.clone();
                    app_config.mpd = mpd_config.clone();
                    app_config.save();
                }
            });
            ui.separator();
            ui.add(egui::Slider::new(&mut params.audio_reactivity_power, 0.0..=20.0).text("Reactivity Power"));
            ui.add(egui::Slider::new(&mut params.particle_count, 10..=200_000).text("Particle Count"));
            ui.add(egui::Slider::new(&mut params.particle_types, 1..=10).text("Particle Types"));
            ui.add(egui::Slider::new(&mut params.slider_animation_speed, 0.0..=5.0).text("Auto-Animate Speed"));
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.attraction_strength, 0.0..=200.0).text("Force Multiplier"));
                animate_selector(ui, &mut params.animate_attraction);
            });
            ui.add(egui::Slider::new(&mut params.time_scale, 0.0..=2.0).text("Time Scale"));
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.dampening, 0.5..=1.0).text("Dampening"));
                animate_selector(ui, &mut params.animate_dampening);
            });
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.min_dist, 0.0..=200.0).text("Repulsion Radius"));
                animate_selector(ui, &mut params.animate_min_dist);
            });
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.interaction_radius, 50.0..=500.0).text("Interaction Radius"));
                animate_selector(ui, &mut params.animate_interaction_radius);
            });
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.density_limit, 0.0..=10.0).text("Density Limit"));
                animate_selector(ui, &mut params.animate_density_limit);
            });
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut params.global_gravity, 0.0..=1.0).text("Global Gravity"));
                animate_selector(ui, &mut params.animate_global_gravity);
            });
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
                    params.global_gravity = rng.gen_range(0.0..0.1);
                }
            });
            
            ui.separator();
            if ui.button("Randomize Everything & Reset").clicked() {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                // Randomize rules
                for i in 0..10 {
                    for j in 0..10 {
                        params.interaction_matrix[i][j] = rng.gen_range(-1.0..1.0);
                    }
                }
                // Randomize proportions
                for i in 0..10 {
                    params.type_proportions[i] = rng.gen_range(0.1..2.0);
                }
                // Randomize world
                params.attraction_strength = rng.gen_range(10.0..150.0);
                params.min_dist = rng.gen_range(5.0..100.0);
                params.interaction_radius = rng.gen_range(50.0..300.0);
                params.density_limit = rng.gen_range(0.1..5.0);
                params.dampening = rng.gen_range(0.8..0.99);
                params.global_gravity = rng.gen_range(0.0..0.1);
                
                // Respawn and trigger time scale animation
                params.spawn_seed = params.spawn_seed.wrapping_add(1);
                params.time_scale = 2.0;
                params.target_time_scale = 0.05;
                params.is_animating_time = true;
            }
            ui.separator();
            
            ui.checkbox(&mut params.continuous_mutation, "Continuous Genetic Mutation");
            
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

fn setup_audio(mut commands: Commands, mpd_config: Res<config::MpdConfig>) {
    let stream_receiver = audio_analysis::start_audio_stream(&mpd_config.fifo_path);
    commands.insert_resource(stream_receiver);

    let (tx, rx) = crossbeam_channel::unbounded();
    commands.insert_resource(MpdState {
        receiver: rx,
        current_song: None,
        album_art: None,
    });

    let host = mpd_config.host.clone();
    std::thread::spawn(move || {
        let mut client = mpd_client::MpdClient::connect(&host);
        let mut last_file = String::new();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Some(ref mut c) = client {
                if let Some(song) = c.get_current_song() {
                    if song.file != last_file {
                        last_file = song.file.clone();
                        let art = c.get_album_art(&song.file);
                        let _ = tx.send((song, art));
                    }
                }
            } else {
                client = mpd_client::MpdClient::connect(&host);
            }
        }
    });

    // Spawn UI root node for MPD info
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(15.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Px(80.0),
                height: Val::Px(80.0),
                display: Display::None,
                ..default()
            },
            ImageNode::default(),
            MpdAlbumArtNode,
        ));
        parent.spawn((
            Text::new("Waiting for MPD..."),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            MpdTextNode,
        ));
    });
}

fn update_audio_stream(mut stream: ResMut<audio_analysis::AudioStreamReceiver>) {
    while let Ok(bands) = stream.receiver.try_recv() {
        stream.current_bands = bands;
    }
}

fn update_mpd_state(
    mut state: ResMut<MpdState>,
    mut images: ResMut<Assets<Image>>,
    mut text_q: Query<&mut Text, With<MpdTextNode>>,
    mut art_q: Query<(&mut ImageNode, &mut Node), With<MpdAlbumArtNode>>,
) {
    if let Ok((song, art_bytes)) = state.receiver.try_recv() {
        state.current_song = Some(song.clone());
        
        let art_handle = if let Some(bytes) = art_bytes {
            if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                let img = Image::from_dynamic(dyn_img, true, bevy::asset::RenderAssetUsages::default());
                Some(images.add(img))
            } else {
                None
            }
        } else {
            None
        };
        state.album_art = art_handle.clone();

        for mut text in &mut text_q {
            text.0 = format!("{}\n{}", song.title, song.artist);
        }

        for (mut ui_image, mut node) in &mut art_q {
            if let Some(h) = &art_handle {
                ui_image.image = h.clone();
                node.display = Display::Flex;
            } else {
                node.display = Display::None;
            }
        }
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
    stream: Option<Res<audio_analysis::AudioStreamReceiver>>,
    time: Res<Time>,
) {
    if params.slider_animation_speed > 0.0 {
        params.slider_animation_time += time.delta_secs() * params.slider_animation_speed;
    }
    
    let wave_sine = (params.slider_animation_time.sin() + 1.0) * 0.5; // 0 to 1
    let reactivity = params.audio_reactivity_power;

    let get_band = |source: AnimateSource| -> Option<f32> {
        match source {
            AnimateSource::Off => None,
            AnimateSource::Sine => Some(wave_sine),
            AnimateSource::LowFreq => stream.as_deref().map(|s| s.current_bands.low * reactivity),
            AnimateSource::MidFreq => stream.as_deref().map(|s| s.current_bands.mid * reactivity),
            AnimateSource::HighFreq => stream.as_deref().map(|s| s.current_bands.high * reactivity),
        }
    };

    if let Some(v) = get_band(params.animate_attraction) {
        params.attraction_strength = v * 200.0;
    }
    if let Some(v) = get_band(params.animate_dampening) {
        params.dampening = 0.5 + v * 0.5;
    }
    if let Some(v) = get_band(params.animate_min_dist) {
        params.min_dist = v * 200.0;
    }
    if let Some(v) = get_band(params.animate_interaction_radius) {
        params.interaction_radius = 50.0 + v * 450.0;
    }
    if let Some(v) = get_band(params.animate_density_limit) {
        params.density_limit = v * 10.0;
    }
    if let Some(v) = get_band(params.animate_global_gravity) {
        params.global_gravity = v * 1.0;
    }

    if params.is_animating_time {
        if params.time_scale > params.target_time_scale {
            params.time_scale -= time.delta_secs() * 0.5;
            if params.time_scale <= params.target_time_scale {
                params.time_scale = params.target_time_scale;
                params.is_animating_time = false;
            }
        } else {
            params.is_animating_time = false;
        }
    }

    // Continuous genetic mutation
    if params.continuous_mutation {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        // Slightly mutate a few random cells in the interaction matrix each frame
        for _ in 0..3 {
            let i = rng.gen_range(0..params.particle_types);
            let j = rng.gen_range(0..params.particle_types);
            let drift = rng.gen_range(-0.01..0.01);
            params.interaction_matrix[i][j] = (params.interaction_matrix[i][j] + drift).clamp(-2.0, 2.0);
        }
    }
}
