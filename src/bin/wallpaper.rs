use bevy::prelude::*;

#[derive(Component)]
pub struct RecordVinyl;

#[derive(Component)]
pub struct RecordSticker;

#[derive(Component)]
struct MvisBar(usize);
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use clap::Parser;

use mvis::audio_analysis;
use mvis::config;
use mvis::gpu_pipeline;
use mvis::instanced_render;
use mvis::mpd_client;
use mvis::params::*;

pub enum MpdEvent {
    NewSong(mpd_client::SongInfo, Option<Vec<u8>>),
    Status(f32, f32), // elapsed, duration
}

#[derive(Resource)]
pub struct MpdState {
    pub receiver: crossbeam_channel::Receiver<MpdEvent>,
    pub current_song: Option<mpd_client::SongInfo>,
    pub album_art: Option<Handle<Image>>,
    pub album_art_colors: Option<[Color; 10]>,
    pub elapsed: f32,
    pub duration: f32,
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    wallpaper: Option<String>,
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    windowed: bool,
}

#[derive(Resource)]
struct AppMode {
    windowed: bool,
}

#[derive(Resource)]
struct DebugConfig {
    enabled: bool,
    timer: Timer,
}

#[derive(Resource)]
pub struct WallpaperData {
    pub path: Option<String>,
    pub colors: Option<[Color; 10]>,
}

#[derive(Component)]
struct BackgroundSprite {
    image_size: Vec2,
}

#[derive(Component)]
struct MpdAlbumArtNode;

#[derive(Component)]
struct MpdTextNode;

#[derive(Component)]
struct MpdRootNode;

fn lock_toggle(ui: &mut egui::Ui, params: &mut SimulationParams, key: &str) {
    let mut is_locked = params.locked_parameters.iter().any(|x| x == key);
    let text = if is_locked { "🔒" } else { "🔓" };
    if ui.toggle_value(&mut is_locked, text).changed() {
        if is_locked {
            params.locked_parameters.push(key.to_string());
        } else {
            params.locked_parameters.retain(|x| x != key);
        }
    }
}

fn animate_selector(ui: &mut egui::Ui, source: &mut AnimateSource) {
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(match *source {
            AnimateSource::Off => "Off",
            AnimateSource::Sine => "Sine Wave",
            AnimateSource::Square => "Square Wave",
            AnimateSource::Triangle => "Triangle Wave",
            AnimateSource::Sawtooth => "Sawtooth Wave",
            AnimateSource::SubBass => "Sub Bass",
            AnimateSource::Bass => "Bass",
            AnimateSource::LowMid => "Low Mid",
            AnimateSource::Mid => "Mid",
            AnimateSource::HighMid => "High Mid",
            AnimateSource::High => "High",
            AnimateSource::Air => "Air",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(source, AnimateSource::Off, "Off");
            ui.selectable_value(source, AnimateSource::Sine, "Sine Wave");
            ui.selectable_value(source, AnimateSource::Square, "Square Wave");
            ui.selectable_value(source, AnimateSource::Triangle, "Triangle Wave");
            ui.selectable_value(source, AnimateSource::Sawtooth, "Sawtooth Wave");
            ui.selectable_value(source, AnimateSource::SubBass, "Sub Bass");
            ui.selectable_value(source, AnimateSource::Bass, "Bass");
            ui.selectable_value(source, AnimateSource::LowMid, "Low Mid");
            ui.selectable_value(source, AnimateSource::Mid, "Mid");
            ui.selectable_value(source, AnimateSource::HighMid, "High Mid");
            ui.selectable_value(source, AnimateSource::High, "High");
            ui.selectable_value(source, AnimateSource::Air, "Air");
        });
}

fn main() {
    let cli = Cli::parse();

    let app_config = config::AppConfig::load_or_create();
    let mut sim_params = app_config.simulation.clone();

    // Toggle UI controls based on windowed mode
    sim_params.show_ui_menu = cli.windowed;

    let mpd_config = app_config.mpd.clone();

    let mut wallpaper_data = WallpaperData {
        path: cli.wallpaper.clone(),
        colors: None,
    };

    // Extract colors from wallpaper synchronously on startup if provided
    if let Some(wallpaper_path) = &cli.wallpaper {
        if let Ok(bytes) = std::fs::read(wallpaper_path) {
            if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                let final_colors = extract_colors(&dyn_img);

                wallpaper_data.colors = Some(final_colors);
                if !sim_params.disable_wallpaper_colors {
                    sim_params.colors = final_colors;
                }
            }
        }
    }

    let mut app = App::new();

    if cli.windowed {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "mvis".to_string(),
                ..default()
            }),
            ..default()
        }));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        }))
        .add_plugins(bevy_live_wallpaper::LiveWallpaperPlugin::default());
    }

    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AppMode {
            windowed: cli.windowed,
        })
        .insert_resource(sim_params)
        .insert_resource(mpd_config)
        .insert_resource(wallpaper_data)
        .insert_resource(DebugConfig {
            enabled: cli.debug,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        })
        .add_plugins((
            gpu_pipeline::GpuPhysicsPlugin,
            instanced_render::InstancedRenderPlugin,
            EguiPlugin::default(),
        ))
        .add_systems(Startup, (setup_camera, setup_audio))
        .add_systems(
            Update,
            (
                camera_movement,
                update_audio_stream,
                update_mpd_state,
                apply_animations,
                update_window_bounds,
                resize_background,
                draw_gravity_wells,
                update_simulation_colors,
                update_mouse_pos,
                update_record_visuals,
                draw_mvis_spectrum,
                debug_memory_usage,
                update_music_ui_layout,
                hot_reload_config,
            ),
        )
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn hot_reload_config(
    mut params: ResMut<SimulationParams>,
    mut local: Local<Option<std::time::SystemTime>>,
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
) {
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(1.0, TimerMode::Repeating));
    }

    if timer.as_mut().unwrap().tick(time.delta()).just_finished() {
        let config_dir = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg_config_home.is_empty() {
                std::path::PathBuf::from(xdg_config_home).join("mvis")
            } else {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".config")
                    .join("mvis")
            }
        } else if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(".config").join("mvis")
        } else {
            std::path::PathBuf::from(".")
        };

        let config_path = config_dir.join("config.toml");
        if let Ok(metadata) = std::fs::metadata(&config_path) {
            if let Ok(modified) = metadata.modified() {
                let reload = match *local {
                    Some(last) => modified > last,
                    None => {
                        *local = Some(modified);
                        false
                    }
                };

                if reload {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(config) = toml::from_str::<config::AppConfig>(&content) {
                            *params = config.simulation;
                            println!("Reloaded config.toml");
                        }
                    }
                    *local = Some(modified);
                }
            }
        }
    }
}

fn update_music_ui_layout(
    params: Res<SimulationParams>,
    mut root_query: Query<&mut Node, With<MpdRootNode>>,
    mut text_query: Query<&mut TextLayout, With<MpdTextNode>>,
) {
    if let Ok(mut node) = root_query.single_mut() {
        let pad_x = Val::Px(params.music_info_padding.x);
        let pad_y = Val::Px(params.music_info_padding.y);

        node.top = Val::Auto;
        node.bottom = Val::Auto;
        node.left = Val::Auto;
        node.right = Val::Auto;

        match params.music_info_anchor {
            MusicInfoAnchor::TopLeft => {
                node.top = pad_y;
                node.left = pad_x;
                node.flex_direction = FlexDirection::Row;
                if let Ok(mut layout) = text_query.single_mut() {
                    layout.justify = Justify::Left;
                }
            }
            MusicInfoAnchor::TopRight => {
                node.top = pad_y;
                node.right = pad_x;
                node.flex_direction = FlexDirection::RowReverse;
                if let Ok(mut layout) = text_query.single_mut() {
                    layout.justify = Justify::Right;
                }
            }
            MusicInfoAnchor::BottomLeft => {
                node.bottom = pad_y;
                node.left = pad_x;
                node.flex_direction = FlexDirection::Row;
                if let Ok(mut layout) = text_query.single_mut() {
                    layout.justify = Justify::Left;
                }
            }
            MusicInfoAnchor::BottomRight => {
                node.bottom = pad_y;
                node.right = pad_x;
                node.flex_direction = FlexDirection::RowReverse;
                if let Ok(mut layout) = text_query.single_mut() {
                    layout.justify = Justify::Right;
                }
            }
        }
    }
}

fn debug_memory_usage(mut debug_config: ResMut<DebugConfig>, time: Res<Time>) {
    if !debug_config.enabled {
        return;
    }

    if debug_config.timer.tick(time.delta()).just_finished() {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pages) = parts[1].parse::<u64>() {
                    // Typical page size on linux is 4096 bytes
                    let page_size = 4096;
                    let rss_mb = (pages * page_size) as f64 / 1024.0 / 1024.0;
                    println!("[DEBUG] Memory Usage (RSS): {:.2} MB", rss_mb);
                }
            }
        }
    }
}

fn normalized_slider_f32(
    ui: &mut egui::Ui,
    value: &mut f32,
    backend_range: std::ops::RangeInclusive<f32>,
) -> egui::Response {
    let min = *backend_range.start();
    let max = *backend_range.end();
    
    // Map to [-1.0, 1.0]
    let mut ui_val = if max == min { 0.0 } else { 2.0 * ((*value - min) / (max - min)) - 1.0 };
    
    let response = ui.add(egui::Slider::new(&mut ui_val, -1.0..=1.0));
    
    if response.changed() {
        // Map back to [min, max]
        *value = min + (max - min) * ((ui_val + 1.0) / 2.0);
    }
    
    response
}

fn ui_system(
    mut contexts: EguiContexts,
    mut params: ResMut<SimulationParams>,
    mpd_config: Res<config::MpdConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    app_mode: Res<AppMode>,
) {
    if !app_mode.windowed {
        return; // Disable menu in wallpaper mode
    }

    if keyboard_input.just_pressed(KeyCode::Tab) || keyboard_input.just_pressed(KeyCode::KeyH) {
        params.show_ui_menu = !params.show_ui_menu;
    }

    if !params.show_ui_menu {
        return;
    }

    if let Ok(ctx) = contexts.ctx_mut() {
        egui::Window::new("Simulation Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Save Config").clicked() {
                    let app_config = config::AppConfig {
                        simulation: params.clone(),
                        mpd: mpd_config.clone(),
                    };
                    app_config.save();
                }
                ui.label("(Press Tab or H to hide)");
            });
            ui.separator();

            draw_global_rules_panel(ui, &mut params);
            draw_visual_effects_panel(ui, &mut params);
            draw_physics_panel(ui, &mut params);
            draw_gravity_wells_panel(ui, &mut params);
            draw_environment_panel(ui, &mut params);
            draw_type_proportions_panel(ui, &mut params);
        });
    }
}

fn draw_environment_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Environment & Basics", |ui| {
        egui::Grid::new("environment_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("");
                ui.label("Particle Count");
                ui.label("");
                ui.add(egui::Slider::new(&mut params.particle_count, 10..=200_000));
                ui.end_row();

                ui.label("");
                ui.label("Particle Types");
                ui.label("");
                ui.add(egui::Slider::new(&mut params.particle_types, 1..=10));
                ui.end_row();

                ui.label("");
                ui.label("Infinite Space (No Bounds)");
                ui.label("");
                ui.checkbox(&mut params.infinite_space, "");
                ui.end_row();

                ui.label("");
                ui.label("Auto-Fit Camera");
                ui.label("");
                ui.checkbox(&mut params.auto_camera, "");
                ui.end_row();
            });
    });
}

fn draw_physics_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Physics & Forces", |ui| {
        egui::Grid::new("physics_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                lock_toggle(ui, params, "time_scale");
                ui.label("Time Scale");
                animate_selector(ui, &mut params.animate_time_scale);
                normalized_slider_f32(ui, &mut params.time_scale, -1.0..=1.0);
                ui.end_row();

                lock_toggle(ui, params, "attraction_strength");
                ui.label("Force Multiplier");
                animate_selector(ui, &mut params.animate_attraction);
                normalized_slider_f32(ui, &mut params.attraction_strength, -120.0..=120.0);
                ui.end_row();

                lock_toggle(ui, params, "dampening");
                ui.label("Dampening");
                animate_selector(ui, &mut params.animate_dampening);
                normalized_slider_f32(ui, &mut params.dampening, 0.8..=1.0);
                ui.end_row();

                lock_toggle(ui, params, "interaction_radius");
                ui.label("Interaction Radius");
                animate_selector(ui, &mut params.animate_interaction_radius);
                normalized_slider_f32(ui, &mut params.interaction_radius, 50.0..=300.0);
                ui.end_row();

                lock_toggle(ui, params, "min_dist");
                ui.label("Repulsion Radius");
                animate_selector(ui, &mut params.animate_min_dist);
                normalized_slider_f32(ui, &mut params.min_dist, 5.0..=100.0);
                ui.end_row();

                lock_toggle(ui, params, "density_limit");
                ui.label("Density Limit");
                animate_selector(ui, &mut params.animate_density_limit);
                normalized_slider_f32(ui, &mut params.density_limit, 0.1..=5.0);
                ui.end_row();

                lock_toggle(ui, params, "global_gravity");
                ui.label("Global Gravity");
                animate_selector(ui, &mut params.animate_global_gravity);
                normalized_slider_f32(ui, &mut params.global_gravity, -0.5..=0.5);
                ui.end_row();
            });
    });
}

fn draw_gravity_wells_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Gravity Wells", |ui| {
        egui::Grid::new("gravity_wells_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                lock_toggle(ui, params, "gravity_well_pattern");
                ui.label("Pattern");
                ui.label("");
                egui::ComboBox::from_id_salt("gravity_pattern")
                    .selected_text(params.gravity_well_pattern.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::None, GravityWellPattern::None.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Ring, GravityWellPattern::Ring.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Grid, GravityWellPattern::Grid.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Line, GravityWellPattern::Line.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Spiral, GravityWellPattern::Spiral.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Star, GravityWellPattern::Star.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Cross, GravityWellPattern::Cross.name());
                        ui.selectable_value(&mut params.gravity_well_pattern, GravityWellPattern::Random, GravityWellPattern::Random.name());
                    });
                ui.end_row();

                ui.label("");
                ui.label("Wells Count");
                ui.label("");
                ui.add(egui::Slider::new(&mut params.gravity_wells, 1..=100));
                ui.end_row();

                lock_toggle(ui, params, "gravity_well_radius");
                ui.label("Radius/Spacing");
                animate_selector(ui, &mut params.animate_gravity_well_radius);
                normalized_slider_f32(ui, &mut params.gravity_well_radius, 0.0..=2000.0);
                ui.end_row();

                lock_toggle(ui, params, "gravity_well_rotation_speed");
                ui.label("Rotation Speed");
                animate_selector(ui, &mut params.animate_gravity_well_rotation);
                normalized_slider_f32(ui, &mut params.gravity_well_rotation_speed, -5.0..=5.0);
                ui.end_row();

                lock_toggle(ui, params, "gravity_well_distance_power");
                ui.label("Outer Well Power Modifier");
                animate_selector(ui, &mut params.animate_gravity_well_distance_power);
                normalized_slider_f32(ui, &mut params.gravity_well_distance_power, -5.0..=5.0);
                ui.end_row();

                ui.label("");
                ui.label("Include Center Well");
                ui.label("");
                ui.checkbox(&mut params.gravity_center_well, "");
                ui.end_row();
            });
    });
}

fn draw_visual_effects_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Visual & Audio Effects", |ui| {
        egui::Grid::new("visual_effects_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("");
                ui.label("Disable Wallpaper Colors");
                ui.label("");
                ui.checkbox(&mut params.disable_wallpaper_colors, "");
                ui.end_row();

                lock_toggle(ui, params, "emission_intensity");
                ui.label("Particle Glow Intensity");
                animate_selector(ui, &mut params.animate_emission_intensity);
                normalized_slider_f32(ui, &mut params.emission_intensity, 0.1..=10.0);
                ui.end_row();

                ui.label("");
                ui.label("Debug Visuals");
                ui.label("");
                ui.checkbox(&mut params.show_debug_visuals, "");
                ui.end_row();

                ui.label("");
                ui.label("Audio Reactivity Power");
                ui.label("");
                normalized_slider_f32(ui, &mut params.audio_reactivity_power, 0.0..=2.0);
                ui.end_row();

                ui.label("");
                ui.label("Auto-Animate Speed");
                animate_selector(ui, &mut params.animate_animation_speed);
                normalized_slider_f32(ui, &mut params.slider_animation_speed, 0.0..=5.0);
                ui.end_row();

                ui.label("");
                ui.label("Follow Mouse");
                ui.label("");
                ui.checkbox(&mut params.follow_mouse, "");
                ui.end_row();

                ui.label("");
                ui.label("Record Exclusion Zone");
                ui.label("");
                ui.checkbox(&mut params.record_exclusion_zone, "");
                ui.end_row();

                if params.record_exclusion_zone {
                    ui.label("");
                    ui.label("Record Radius");
                    animate_selector(ui, &mut params.animate_record_radius);
                    normalized_slider_f32(ui, &mut params.record_radius, 50.0..=1000.0);
                    ui.end_row();

                    ui.label("");
                    ui.label("Record Rotation Speed");
                    animate_selector(ui, &mut params.animate_record_rotation_speed);
                    normalized_slider_f32(ui, &mut params.record_rotation_speed, -10.0..=10.0);
                    ui.end_row();
                }

                ui.label("");
                ui.label("Show osu!mvis Spectrum");
                ui.label("");
                ui.checkbox(&mut params.show_mvis_spectrum, "");
                ui.end_row();

                if params.show_mvis_spectrum {
                    ui.label("");
                    ui.label("Spectrum Height");
                    animate_selector(ui, &mut params.animate_mvis_spectrum_height);
                    normalized_slider_f32(ui, &mut params.mvis_spectrum_height, 10.0..=500.0);
                    ui.end_row();

                    ui.label("");
                    ui.label("Bar Thickness");
                    animate_selector(ui, &mut params.animate_mvis_bar_thickness);
                    normalized_slider_f32(ui, &mut params.mvis_bar_thickness, 0.5..=20.0);
                    ui.end_row();

                    ui.label("");
                    ui.label("Spectrum Repeats");
                    ui.label("");
                    ui.add(egui::Slider::new(&mut params.mvis_repeat_count, 1..=8));
                    ui.end_row();
                }
            });
    });
}

fn draw_global_rules_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Randomizer & Ecosystem", |ui| {
        egui::Grid::new("global_rules_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("");
                ui.label("Continuous Genetic Mutation");
                ui.label("");
                ui.checkbox(&mut params.continuous_mutation, "");
                ui.end_row();

                lock_toggle(ui, params, "matrix_base");
                ui.label("Matrix Random Base");
                ui.label("");
                normalized_slider_f32(ui, &mut params.matrix_base, -1.0..=1.0);
                ui.end_row();

                lock_toggle(ui, params, "matrix_spread");
                ui.label("Matrix Random Spread");
                ui.label("");
                normalized_slider_f32(ui, &mut params.matrix_spread, 0.0..=1.0);
                ui.end_row();

                ui.label("");
                ui.label("Lock Rules (Matrix & Proportions)");
                ui.label("");
                ui.checkbox(&mut params.lock_rules, "");
                ui.end_row();

                ui.label("");
                ui.label("Lock Environment (Forces & Radiuses)");
                ui.label("");
                ui.checkbox(&mut params.lock_environment, "");
                ui.end_row();

                ui.label("");
                ui.label("Lock Gravity Wells");
                ui.label("");
                ui.checkbox(&mut params.lock_gravity_wells, "");
                ui.end_row();

                ui.label("");
                ui.label("Lock Audio Reactivity");
                ui.label("");
                ui.checkbox(&mut params.lock_audio_reactivity, "");
                ui.end_row();
            });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Randomize Rules").clicked() && !params.lock_rules {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let base = params.matrix_base;
                let spread = params.matrix_spread;
                for i in 0..10 {
                    for j in 0..10 {
                        params.interaction_matrix[i][j] =
                            rng.gen_range((base - spread)..=(base + spread));
                    }
                }
            }
            if ui.button("Randomize Proportions").clicked() && !params.lock_rules {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                for i in 0..10 {
                    params.type_proportions[i] = rng.gen_range(0.1..2.0);
                }
                params.spawn_seed = params.spawn_seed.wrapping_add(1);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Randomize World").clicked() {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                macro_rules! randomize {
                    ($field:ident, $range:expr) => {
                        if !params.locked_parameters.contains(&stringify!($field).to_string()) {
                            params.$field = rng.gen_range($range);
                        }
                    }
                }

                // Environment
                randomize!(attraction_strength, 10.0..100.0);
                randomize!(min_dist, 10.0..80.0);
                randomize!(interaction_radius, 50.0..250.0);
                randomize!(density_limit, 0.2..3.0);
                randomize!(dampening, 0.85..0.98);
                randomize!(global_gravity, 0.0..0.05);
            }
        });
    });
}

fn draw_type_proportions_panel(ui: &mut egui::Ui, params: &mut SimulationParams) {
    ui.collapsing("Type Proportions & Matrices", |ui| {
        ui.collapsing("Proportions", |ui| {
            for i in 0..params.particle_types {
                ui.horizontal(|ui| {
                    ui.label(format!("Type {}", i));
                    normalized_slider_f32(ui, &mut params.type_proportions[i], 0.0..=5.0);
                });
            }
            if ui.button("Apply Proportions (Respawn)").clicked() {
                params.spawn_seed = params.spawn_seed.wrapping_add(1);
            }
        });

        ui.collapsing("Interaction Matrix", |ui| {
            egui::Grid::new("interaction_matrix_grid").show(ui, |ui| {
                ui.label("");
                for j in 0..params.particle_types {
                    ui.label(format!("T{}", j));
                }
                ui.end_row();

                for i in 0..params.particle_types {
                    ui.label(format!("Type {}", i));
                    for j in 0..params.particle_types {
                        ui.add(
                            egui::DragValue::new(&mut params.interaction_matrix[i][j])
                                .speed(0.01)
                                .range(-1.0..=1.0),
                        );
                    }
                    ui.end_row();
                }
            });
        });
    });
}

fn setup_audio(mut commands: Commands, mpd_config: Res<config::MpdConfig>) {
    let stream_receiver = audio_analysis::start_audio_stream(&mpd_config.fifo_path);
    commands.insert_resource(stream_receiver);

    let (tx, rx) = crossbeam_channel::unbounded();
    commands.insert_resource(MpdState {
        receiver: rx,
        current_song: None,
        album_art: None,
        album_art_colors: None,
        elapsed: 0.0,
        duration: 0.0,
    });

    let host = mpd_config.host.clone();
    std::thread::spawn(move || {
        let mut client = mpd_client::MpdClient::connect(&host);
        let mut last_file = String::new();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Some(ref mut c) = client {
                if let Some(status) = c.get_status() {
                    let _ = tx.send(MpdEvent::Status(status.0, status.1));
                }

                if let Some(song) = c.get_current_song() {
                    if song.file != last_file {
                        last_file = song.file.clone();
                        let art = c.get_album_art(&song.file);
                        let _ = tx.send(MpdEvent::NewSong(song, art));
                    }
                }
            } else {
                client = mpd_client::MpdClient::connect(&host);
            }
        }
    });

    // Spawn UI root node for MPD info
    commands
        .spawn((
            MpdRootNode,
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
        ))
        .with_children(|parent| {
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

fn update_mouse_pos(
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut params: ResMut<SimulationParams>,
    time: Res<Time>,
) {
    if params.follow_mouse {
        if let (Ok(window), Ok((camera, camera_transform))) =
            (window_query.single(), camera_query.single())
        {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    params.target_mouse_pos = world_pos;
                }
            }
        }
    } else {
        params.target_mouse_pos = Vec2::ZERO;
    }

    let lerp_factor = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
    params.mouse_pos = params.mouse_pos.lerp(params.target_mouse_pos, lerp_factor);
}

// TODO: Factor complex query into a type definition
#[allow(clippy::type_complexity)]
fn update_record_visuals(
    params: Res<SimulationParams>,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut vinyl_query: Query<
        (&mut Transform, &mut Visibility),
        (With<RecordVinyl>, Without<RecordSticker>),
    >,
    mut sticker_query: Query<(&mut Transform, &mut Visibility), With<RecordSticker>>,
) {
    let is_active = params.record_exclusion_zone;
    let scale = params.record_radius;
    let pos = params.mouse_pos;

    // Spin rate based on dedicated rotation parameter
    let spin = time.elapsed_secs() * params.record_rotation_speed;

    if let Ok((mut transform, mut visibility)) = vinyl_query.single_mut() {
        *visibility = if is_active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        transform.scale = Vec3::splat(scale);
        transform.rotation = Quat::from_rotation_z(spin);
    }
    if let Ok((mut transform, mut visibility)) = sticker_query.single_mut() {
        *visibility = if is_active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
        transform.scale = Vec3::splat(scale * 0.4); // Sticker is 40% the size of the vinyl
        transform.rotation = Quat::from_rotation_z(spin);
    }

    // Draw concentric grooves (ribs) on the record using Gizmos
    if is_active {
        let num_grooves = 12;
        let groove_color = Color::srgba(0.08, 0.08, 0.08, 0.8); // Slightly lighter than the record base (0.05)

        // The sticker takes up the inner 40% (scale * 0.4). So ribs go from 45% to 95%.
        let min_r = scale * 0.45;
        let max_r = scale * 0.95;

        for i in 0..num_grooves {
            let t = i as f32 / (num_grooves - 1) as f32;
            let r = min_r + t * (max_r - min_r);
            gizmos.circle_2d(pos, r, groove_color);
        }
    }
}

fn draw_mvis_spectrum(
    mut commands: Commands,
    params: Res<SimulationParams>,
    time: Res<Time>,
    stream: Res<audio_analysis::AudioStreamReceiver>,
    mpd_state: Res<MpdState>,
    mut bar_query: Query<(Entity, &mut Transform, &mut Sprite, &MvisBar)>,
) {
    let num_bands = 128;
    let total_bars = num_bands * params.mvis_repeat_count;

    // Check if we need to despawn and recreate bars
    let count = bar_query.iter().count();
    let should_exist = params.record_exclusion_zone && params.show_mvis_spectrum;

    if !should_exist || count != total_bars {
        for (entity, _, _, _) in &bar_query {
            commands.entity(entity).despawn();
        }

        if should_exist {
            for i in 0..total_bars {
                commands.spawn((
                    Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(0.0, params.mvis_bar_thickness)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, -0.45),
                    MvisBar(i),
                ));
            }
        }
        return;
    }

    let radius = params.record_radius;
    let center = params.mouse_pos;
    let spin = time.elapsed_secs() * params.record_rotation_speed;

    let colors = mpd_state.album_art_colors.unwrap_or([Color::WHITE; 10]);

    for (_, mut transform, mut sprite, bar) in &mut bar_query {
        let i = bar.0;
        let angle = spin + (i as f32 / total_bars as f32) * std::f32::consts::TAU;
        let dir = Vec2::new(angle.cos(), angle.sin());

        let cycle_idx = i % num_bands;
        let spec_idx = if cycle_idx < 64 {
            cycle_idx
        } else {
            127 - cycle_idx
        };
        let magnitude = stream.current_bands.spectrum[spec_idx];

        let bar_len = (magnitude * params.mvis_spectrum_height).max(1.0);

        sprite.custom_size = Some(Vec2::new(bar_len, params.mvis_bar_thickness));

        // Offset center of the sprite because sprites grow outward from their center
        let offset = dir * (radius + bar_len * 0.5);
        transform.translation = (center + offset).extend(-0.45);
        transform.rotation = Quat::from_rotation_z(angle);

        let color_idx = (i * 10 / total_bars) % 10;
        sprite.color = colors[color_idx];
    }
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
    mut materials: ResMut<Assets<ColorMaterial>>,
    sticker_query: Query<&MeshMaterial2d<ColorMaterial>, With<RecordSticker>>,
) {
    while let Ok(event) = state.receiver.try_recv() {
        match event {
            MpdEvent::Status(elapsed, duration) => {
                state.elapsed = elapsed;
                state.duration = duration;
            }
            MpdEvent::NewSong(song, art_bytes) => {
                state.current_song = Some(song.clone());

                let art_handle = if let Some(bytes) = art_bytes {
                    if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                        // Extract up to 10 distinct, vibrant colors from the image
                        let final_colors = extract_colors(&dyn_img);
                        state.album_art_colors = Some(final_colors);
                        let img = Image::from_dynamic(
                            dyn_img,
                            true,
                            bevy::asset::RenderAssetUsages::default(),
                        );
                        Some(images.add(img))
                    } else {
                        None
                    }
                } else {
                    None
                };
                state.album_art = art_handle.clone();

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
    }

    if let Some(song) = &state.current_song {
        for mut text in &mut text_q {
            let el_m = (state.elapsed / 60.0) as u32;
            let el_s = (state.elapsed % 60.0) as u32;
            let du_m = (state.duration / 60.0) as u32;
            let du_s = (state.duration % 60.0) as u32;
            text.0 = format!(
                "{}\n{}\n{:02}:{:02} / {:02}:{:02}",
                song.title, song.artist, el_m, el_s, du_m, du_s
            );
        }
    }

    // Always ensure the record sticker material is up to date with the current album art
    if let Ok(material_handle) = sticker_query.single() {
        if let Some(mat) = materials.get_mut(material_handle.id()) {
            mat.texture = state.album_art.clone();
        }
    }
}

fn setup_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    wallpaper: Res<WallpaperData>,
    app_mode: Res<AppMode>,
) {
    let mut camera_cmds = commands.spawn((
        Camera2d,
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        bevy::render::view::Hdr,
        bevy::post_process::bloom::Bloom::default(),
        bevy::render::view::Msaa::Off,
        Transform::from_scale(Vec3::splat(2.0)),
    ));

    if !app_mode.windowed {
        camera_cmds.insert(bevy_live_wallpaper::LiveWallpaperCamera);
    }

    let camera = camera_cmds.id();

    if let Some(path) = &wallpaper.path {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                let img_width = dyn_img.width() as f32;
                let img_height = dyn_img.height() as f32;
                let img =
                    Image::from_dynamic(dyn_img, true, bevy::asset::RenderAssetUsages::default());
                let handle = images.add(img);

                let bg = commands
                    .spawn((
                        Sprite {
                            image: handle,
                            custom_size: Some(Vec2::new(img_width, img_height)),
                            color: Color::srgba(1.0, 1.0, 1.0, 0.5), // Dim the background slightly
                            ..default()
                        },
                        Transform::from_xyz(0.0, 0.0, -100.0),
                        BackgroundSprite {
                            image_size: Vec2::new(img_width, img_height),
                        },
                    ))
                    .id();

                commands.entity(camera).add_children(&[bg]);
            }
        }
    }

    // Spawn the vinyl record (large black circle)
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(0.05, 0.05, 0.05, 1.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.5)),
        Visibility::Hidden,
        RecordVinyl,
    ));

    // Spawn the sticker (smaller circle, initially white so album art texture is visible)
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 1.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.4)),
        Visibility::Hidden,
        RecordSticker,
    ));
}

fn camera_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut params: ResMut<SimulationParams>,
) {
    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };
    let mut direction = Vec3::ZERO;
    let mut zoom_delta = 0.0;

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

    if keyboard_input.pressed(KeyCode::KeyQ) {
        zoom_delta += 1.5 * time.delta_secs();
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        zoom_delta -= 1.5 * time.delta_secs();
    }

    // Disable auto camera if user manually pans or zooms
    if direction.length_squared() > 0.0 || zoom_delta != 0.0 {
        params.auto_camera = false;
    }

    if params.auto_camera {
        if let Ok(window) = window_query.single() {
            let target_scale_x = (params.region_size.x * 1.05) / window.width().max(1.0);
            let target_scale_y = (params.region_size.y * 1.05) / window.height().max(1.0);
            let target_scale = target_scale_x.max(target_scale_y).max(0.1);

            let lerp_factor = (5.0 * time.delta_secs()).min(1.0);
            transform.translation = transform.translation.lerp(Vec3::ZERO, lerp_factor);

            let current_scale = transform.scale.x;
            let new_scale = current_scale + (target_scale - current_scale) * lerp_factor;
            transform.scale = Vec3::splat(new_scale);
        }
    } else {
        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
        }
        let speed = 500.0 * transform.scale.x;
        transform.translation += direction * speed * time.delta_secs();

        if zoom_delta != 0.0 {
            transform.scale *= 1.0 + zoom_delta;
        }
    }
}

fn apply_animations(
    mut params: ResMut<SimulationParams>,
    stream: Option<Res<audio_analysis::AudioStreamReceiver>>,
    time: Res<Time>,
) {
    let mut time_advance = time.delta_secs();
    let dt = time.delta_secs();

    // Calculate smoothed audio energy envelope
    if let Some(stream_ref) = &stream {
        let bands = &stream_ref.current_bands;
        let total_energy = (bands.sub_bass
            + bands.bass
            + bands.low_mid
            + bands.mid
            + bands.high_mid
            + bands.high
            + bands.air)
            / 7.0;

        let attack = 15.0;
        let release = 2.0;
        if total_energy > params.smoothed_audio_energy {
            params.smoothed_audio_energy +=
                (total_energy - params.smoothed_audio_energy) * (attack * dt).min(1.0);
        } else {
            params.smoothed_audio_energy +=
                (total_energy - params.smoothed_audio_energy) * (release * dt).min(1.0);
        }

        // Use smoothed energy for frequency modulation (speed up time gracefully)
        time_advance *= 1.0 + (params.smoothed_audio_energy * params.audio_reactivity_power * 10.0);
    } else {
        // Decay to 0 if no audio
        params.smoothed_audio_energy -= params.smoothed_audio_energy * (2.0 * dt).min(1.0);
    }

    let reactivity = params.audio_reactivity_power;
    let t = params.slider_animation_time;
    let wave_sine = (t.sin() + 1.0) * 0.5; // 0 to 1
    let wave_square = if t.sin() > 0.0 { 1.0 } else { 0.0 };
    let wave_triangle = 1.0 - (2.0 * (t / std::f32::consts::TAU).fract() - 1.0).abs();
    let wave_sawtooth = (t / std::f32::consts::TAU).fract();

    // Amplitude coupling factor
    // When reactivity is 0, waves are pure mathematical LFOs (mult = 1.0)
    // When reactivity > 0, they pulse in intensity with the music envelope
    let audio_mult = 1.0 + (params.smoothed_audio_energy * reactivity * 2.0);

    let get_band = |source: AnimateSource| -> Option<f32> {
        match source {
            AnimateSource::Off => None,
            AnimateSource::Sine => Some(wave_sine * audio_mult),
            AnimateSource::Square => Some(wave_square * audio_mult),
            AnimateSource::Triangle => Some(wave_triangle * audio_mult),
            AnimateSource::Sawtooth => Some(wave_sawtooth * audio_mult),
            AnimateSource::SubBass => stream.as_deref().map(|s| s.current_bands.sub_bass),
            AnimateSource::Bass => stream.as_deref().map(|s| s.current_bands.bass),
            AnimateSource::LowMid => stream.as_deref().map(|s| s.current_bands.low_mid),
            AnimateSource::Mid => stream.as_deref().map(|s| s.current_bands.mid),
            AnimateSource::HighMid => stream.as_deref().map(|s| s.current_bands.high_mid),
            AnimateSource::High => stream.as_deref().map(|s| s.current_bands.high),
            AnimateSource::Air => stream.as_deref().map(|s| s.current_bands.air),
        }
    };

    if let Some(v) = get_band(params.animate_animation_speed) {
        params.slider_animation_speed = (v * 5.0 * reactivity).clamp(0.0, 5.0);
    }

    if params.slider_animation_speed > 0.0 {
        params.slider_animation_time += time_advance * params.slider_animation_speed;
    }

    if let Some(v) = get_band(params.animate_attraction) {
        params.attraction_strength = (v * 100.0 * reactivity).clamp(0.0, 200.0);
    }
    if let Some(v) = get_band(params.animate_dampening) {
        params.dampening = (0.5 + (v * 0.5 * reactivity)).clamp(0.5, 1.0);
    }
    if let Some(v) = get_band(params.animate_min_dist) {
        params.min_dist = (v * 100.0 * reactivity).clamp(0.0, 200.0);
    }
    if let Some(v) = get_band(params.animate_interaction_radius) {
        params.interaction_radius = (50.0 + (v * 300.0 * reactivity)).clamp(50.0, 500.0);
    }
    if let Some(v) = get_band(params.animate_density_limit) {
        params.density_limit = (v * 5.0 * reactivity).clamp(0.0, 10.0);
    }
    if let Some(v) = get_band(params.animate_global_gravity) {
        params.global_gravity = (v * 0.5 * reactivity).clamp(-0.5, 0.5);
    }
    if let Some(v) = get_band(params.animate_gravity_well_rotation) {
        params.gravity_well_rotation_speed = (v * 5.0 * reactivity).clamp(-5.0, 5.0);
    }
    params.gravity_well_rotation += params.gravity_well_rotation_speed * time_advance;
    if let Some(v) = get_band(params.animate_gravity_well_distance_power) {
        params.gravity_well_distance_power = (v * 5.0 * reactivity).clamp(-5.0, 5.0);
    }
    if let Some(v) = get_band(params.animate_time_scale) {
        params.time_scale = (v * 1.0 * reactivity).clamp(0.0, 2.0);
    }
    if let Some(v) = get_band(params.animate_gravity_well_radius) {
        params.gravity_well_radius = (v * 2000.0 * reactivity).clamp(0.0, 2000.0);
    }
    if let Some(v) = get_band(params.animate_emission_intensity) {
        params.emission_intensity = (0.1 + (v * 10.0 * reactivity)).clamp(0.1, 10.0);
    }
    if let Some(v) = get_band(params.animate_record_radius) {
        params.record_radius = (50.0 + (v * 1000.0 * reactivity)).clamp(50.0, 1000.0);
    }
    if let Some(v) = get_band(params.animate_record_rotation_speed) {
        params.record_rotation_speed = (v * 10.0 * reactivity).clamp(-10.0, 10.0);
    }
    if let Some(v) = get_band(params.animate_mvis_spectrum_height) {
        params.mvis_spectrum_height = (10.0 + (v * 500.0 * reactivity)).clamp(10.0, 500.0);
    }
    if let Some(v) = get_band(params.animate_mvis_bar_thickness) {
        params.mvis_bar_thickness = (0.5 + (v * 20.0 * reactivity)).clamp(0.5, 20.0);
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
            params.interaction_matrix[i][j] =
                (params.interaction_matrix[i][j] + drift).clamp(-2.0, 2.0);
        }
    }
}

fn update_window_bounds(
    mut params: ResMut<SimulationParams>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    for window in window_query.iter() {
        let w = window.width();
        let h = window.height();
        if w > 0.0 && h > 0.0 {
            params.region_size = Vec2::new(w, h);
        }
    }
}

fn resize_background(
    mut query: Query<(&mut Sprite, &BackgroundSprite)>,
    camera_query: Query<&Transform, With<Camera>>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if let Ok(window) = window_query.single() {
        if let Ok(camera_transform) = camera_query.single() {
            let w = window.width();
            let h = window.height();

            for (mut sprite, bg) in &mut query {
                let scale_x = (w * camera_transform.scale.x) / bg.image_size.x;
                let scale_y = (h * camera_transform.scale.y) / bg.image_size.y;
                let scale = scale_x.max(scale_y);

                sprite.custom_size = Some(bg.image_size * scale);
            }
        }
    }
}

fn draw_gravity_wells(mut gizmos: Gizmos, params: Res<SimulationParams>) {
    if !params.show_debug_visuals {
        return;
    }

    let num_wells = params.gravity_wells;
    if num_wells == 0 && !params.gravity_center_well {
        return;
    }

    if params.gravity_center_well {
        gizmos.circle_2d(params.mouse_pos, 15.0, Color::srgba(1.0, 1.0, 1.0, 0.4));
        gizmos.circle_2d(params.mouse_pos, 5.0, Color::srgba(1.0, 1.0, 1.0, 0.8));
    }

    if num_wells == 0 || params.gravity_well_pattern == GravityWellPattern::None {
        return;
    }

    let rot_cos = params.gravity_well_rotation.cos();
    let rot_sin = params.gravity_well_rotation.sin();

    for i in 0..num_wells {
        let mut well_pos = match params.gravity_well_pattern {
            GravityWellPattern::None => unreachable!(),
            GravityWellPattern::Ring => {
                let angle = (i as f32 / num_wells as f32) * std::f32::consts::TAU;
                Vec2::new(angle.cos(), angle.sin()) * params.gravity_well_radius
            }
            GravityWellPattern::Grid => {
                let cols = (num_wells as f32).sqrt().ceil() as u32;
                let cols = cols.max(1);
                let row = i / cols;
                let col = i % cols;
                let rows = num_wells.div_ceil(cols);
                let rows = rows.max(1);

                let offset_x = (col as f32 - (cols - 1) as f32 * 0.5) * params.gravity_well_radius;
                let offset_y = (row as f32 - (rows - 1) as f32 * 0.5) * params.gravity_well_radius;
                Vec2::new(offset_x, offset_y)
            }
            GravityWellPattern::Line => {
                let offset_x =
                    (i as f32 - (num_wells - 1) as f32 * 0.5) * params.gravity_well_radius;
                Vec2::new(offset_x, 0.0)
            }
            GravityWellPattern::Spiral => {
                let angle = (i as f32) * std::f32::consts::PI * 1.5;
                let r = (i as f32 + 1.0) * (params.gravity_well_radius * 0.2);
                Vec2::new(angle.cos(), angle.sin()) * r
            }
            GravityWellPattern::Star => {
                let angle = (i as f32 / num_wells as f32) * std::f32::consts::TAU;
                let r = if i % 2 == 0 {
                    params.gravity_well_radius
                } else {
                    params.gravity_well_radius * 0.4
                };
                Vec2::new(angle.cos(), angle.sin()) * r
            }
            GravityWellPattern::Cross => {
                let arms = 4;
                let points_per_arm = num_wells.div_ceil(arms);
                let arm_idx = i % arms;
                let point_idx = i / arms;

                let angle = (arm_idx as f32 / arms as f32) * std::f32::consts::TAU;
                let r =
                    ((point_idx as f32 + 1.0) / points_per_arm as f32) * params.gravity_well_radius;
                Vec2::new(angle.cos(), angle.sin()) * r
            }
            GravityWellPattern::Random => {
                // Use a seeded hash of the index + spawn_seed to keep it stable
                let hash1 = i.wrapping_mul(374761393).wrapping_add(params.spawn_seed);
                let hash2 = i.wrapping_mul(668265263).wrapping_add(params.spawn_seed);
                let rand_angle = (hash1 as f32 / u32::MAX as f32) * std::f32::consts::TAU;
                let rand_r = (hash2 as f32 / u32::MAX as f32) * params.gravity_well_radius;
                Vec2::new(rand_angle.cos(), rand_angle.sin()) * rand_r
            }
        };

        let rx = well_pos.x * rot_cos - well_pos.y * rot_sin;
        let ry = well_pos.x * rot_sin + well_pos.y * rot_cos;
        well_pos = Vec2::new(rx, ry) + params.mouse_pos;

        let dist_from_center = well_pos.distance(params.mouse_pos);
        let power_mult = 1.0 + (dist_from_center * 0.01) * params.gravity_well_distance_power;

        let alpha = (0.2 * power_mult.abs()).clamp(0.0, 1.0);
        let color = if power_mult < 0.0 {
            Color::srgba(1.0, 0.0, 0.0, alpha)
        } else {
            Color::srgba(1.0, 1.0, 1.0, alpha)
        };

        gizmos.circle_2d(well_pos, 10.0 * power_mult.abs().clamp(0.1, 5.0), color);
        gizmos.line_2d(
            well_pos - Vec2::new(5.0, 0.0),
            well_pos + Vec2::new(5.0, 0.0),
            Color::srgba(1.0, 1.0, 1.0, (alpha + 0.2).clamp(0.0, 1.0)),
        );
        gizmos.line_2d(
            well_pos - Vec2::new(0.0, 5.0),
            well_pos + Vec2::new(0.0, 5.0),
            Color::srgba(1.0, 1.0, 1.0, (alpha + 0.2).clamp(0.0, 1.0)),
        );
    }
}

fn update_simulation_colors(
    mut params: ResMut<SimulationParams>,
    wallpaper: Res<WallpaperData>,
    mpd_state: Res<MpdState>,
) {
    if !params.disable_wallpaper_colors && wallpaper.colors.is_some() {
        if let Some(c) = wallpaper.colors {
            params.colors = c;
        }
    } else if let Some(album_colors) = mpd_state.album_art_colors {
        params.colors = album_colors;
    }
}

fn extract_colors(dyn_img: &image::DynamicImage) -> [Color; 10] {
    let img_resized = dyn_img.resize_exact(32, 32, image::imageops::FilterType::Triangle);
    let mut pixels: Vec<_> = img_resized.to_rgba8().pixels().map(|p| p.0).collect();

    pixels.sort_by(|a, b| {
        let max_a = a[0].max(a[1]).max(a[2]) as f32;
        let min_a = a[0].min(a[1]).min(a[2]) as f32;
        let sat_a = if max_a == 0.0 {
            0.0
        } else {
            (max_a - min_a) / max_a
        };

        let max_b = b[0].max(b[1]).max(b[2]) as f32;
        let min_b = b[0].min(b[1]).min(b[2]) as f32;
        let sat_b = if max_b == 0.0 {
            0.0
        } else {
            (max_b - min_b) / max_b
        };

        sat_b
            .partial_cmp(&sat_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut chosen_colors = Vec::new();
    let mut target_dist = 0.5;

    while chosen_colors.len() < 10 && target_dist >= 0.0 {
        for p in &pixels {
            let color = Color::srgba(
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
                1.0,
            );

            let mut similar = false;
            for c in &chosen_colors {
                let c: Color = *c;
                let srgba1 = color.to_srgba();
                let srgba2 = c.to_srgba();
                let dist = (srgba1.red - srgba2.red).abs()
                    + (srgba1.green - srgba2.green).abs()
                    + (srgba1.blue - srgba2.blue).abs();
                if dist < target_dist {
                    similar = true;
                    break;
                }
            }

            if !similar {
                chosen_colors.push(color);
                if chosen_colors.len() == 10 {
                    break;
                }
            }
        }
        target_dist -= 0.05;
    }

    let mut final_colors = [Color::WHITE; 10];
    for i in 0..10 {
        if i < chosen_colors.len() {
            final_colors[i] = chosen_colors[i];
        } else if !chosen_colors.is_empty() {
            final_colors[i] = chosen_colors[i % chosen_colors.len()];
        }
    }

    final_colors
}
