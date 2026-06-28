use bevy::prelude::*;

#[derive(Component)]
pub struct RecordVinyl;

#[derive(Component)]
pub struct RecordSticker;

use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use clap::Parser;

use mvis::audio_analysis;
use mvis::config;
#[derive(Component)]
pub struct MvisBar(usize);

#[derive(Component)]
pub struct MvisBarCap(usize);

#[derive(Resource)]
pub struct BarCapTexture(pub Handle<Image>);
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
    #[arg(long, value_enum)]
    mode: Option<mvis::params::WallpaperMode>,
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    windowed: bool,
    #[arg(long)]
    width: Option<f32>,
    #[arg(long)]
    height: Option<f32>,
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


fn param_row_ui(ui: &mut egui::Ui, params: &mut SimulationParams, param: mvis::params::FloatParam) {
    let meta = param.meta();
    
    ui.label("");

    ui.label(meta.name);

    let mut val = param.get_val(params);
    if normalized_slider_f32(ui, &mut val, meta.slider_range.clone()).changed() {
        param.set_val(params, val);
    }
    
    ui.end_row();
}

fn main() {
    let cli = Cli::parse();

    let app_config = config::AppConfig::load_or_create();
    let mut sim_params = app_config.simulation.clone();

    if let Some(w) = cli.width {
        sim_params.region_size.x = w;
    }
    if let Some(h) = cli.height {
        sim_params.region_size.y = h;
    }
    if let Some(m) = cli.mode {
        sim_params.wallpaper_mode = m;
    }

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
                resolution: bevy::window::WindowResolution::new(
                    sim_params.region_size.x as u32,
                    sim_params.region_size.y as u32,
                ),
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
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, (setup_camera, setup_audio))
        .add_systems(
            Update,
            (
                update_audio_stream,
                update_mpd_state,
                update_window_bounds,
                resize_background,
                update_record_visuals,
                draw_mvis_spectrum,
                update_simulation_colors,

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
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera>>,
    mut root_query: Query<(Entity, &mut Node, Option<&UiTargetCamera>), With<MpdRootNode>>,
    mut text_query: Query<&mut TextLayout, With<MpdTextNode>>,
) {
    if let Ok((entity, mut node, target_camera)) = root_query.single_mut() {
        if target_camera.is_none() {
            if let Ok(camera_entity) = camera_query.single() {
                commands.entity(entity).insert(UiTargetCamera(camera_entity));
            }
        }

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
    mut preset_name_input: Local<String>,
    mut selected_preset: Local<String>,
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

            draw_presets_panel(ui, &mut params, &mpd_config, &mut preset_name_input, &mut selected_preset);

            draw_visual_effects_panel(ui, &mut params);
        });
    }
}

fn draw_presets_panel(
    ui: &mut egui::Ui,
    params: &mut SimulationParams,
    mpd_config: &config::MpdConfig,
    preset_name_input: &mut String,
    selected_preset: &mut String,
) {
    ui.collapsing("Presets", |ui| {
        let presets = config::AppConfig::list_presets();

        ui.horizontal(|ui| {
            ui.label("Load Preset:");
            egui::ComboBox::from_id_salt("load_preset_combo")
                .selected_text(selected_preset.as_str())
                .show_ui(ui, |ui| {
                    for preset in &presets {
                        ui.selectable_value(selected_preset, preset.clone(), preset);
                    }
                });

            if ui.button("Load").clicked() && !selected_preset.is_empty() {
                if let Some(loaded_config) = config::AppConfig::load_preset(selected_preset) {
                    *params = loaded_config.simulation;
                    // Automatically save to config.toml so it's the active config
                    let new_active_config = config::AppConfig {
                        simulation: params.clone(),
                        mpd: mpd_config.clone(),
                    };
                    new_active_config.save();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Save Preset:");
            ui.text_edit_singleline(preset_name_input);
            if ui.button("Save").clicked() && !preset_name_input.is_empty() {
                let app_config = config::AppConfig {
                    simulation: params.clone(),
                    mpd: mpd_config.clone(),
                };
                app_config.save_preset(preset_name_input);
                *selected_preset = preset_name_input.clone();
                preset_name_input.clear();
            }
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


                ui.label("");
                ui.label("Record Exclusion Zone");
                ui.label("");
                ui.checkbox(&mut params.record_exclusion_zone, "");
                ui.end_row();

                ui.label("");
                ui.label("Bar Layout");
                ui.label("");
                egui::ComboBox::from_id_salt("bar_layout")
                    .selected_text(params.bar_layout.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.bar_layout, BarLayout::Circular, BarLayout::Circular.name());
                        ui.selectable_value(&mut params.bar_layout, BarLayout::Top, BarLayout::Top.name());
                        ui.selectable_value(&mut params.bar_layout, BarLayout::Bottom, BarLayout::Bottom.name());
                    });
                ui.end_row();

                ui.label("");
                ui.label("Wallpaper Mode");
                ui.label("");
                egui::ComboBox::from_id_salt("wallpaper_mode")
                    .selected_text(params.wallpaper_mode.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.wallpaper_mode, mvis::params::WallpaperMode::Zoom, mvis::params::WallpaperMode::Zoom.name());
                        ui.selectable_value(&mut params.wallpaper_mode, mvis::params::WallpaperMode::Fit, mvis::params::WallpaperMode::Fit.name());
                        ui.selectable_value(&mut params.wallpaper_mode, mvis::params::WallpaperMode::Stretch, mvis::params::WallpaperMode::Stretch.name());
                        ui.selectable_value(&mut params.wallpaper_mode, mvis::params::WallpaperMode::Center, mvis::params::WallpaperMode::Center.name());
                    });
                ui.end_row();

                ui.label("");
                ui.label("Music Info Anchor");
                ui.label("");
                let current_anchor_str = match params.music_info_anchor {
                    mvis::params::MusicInfoAnchor::TopLeft => "Top Left",
                    mvis::params::MusicInfoAnchor::TopRight => "Top Right",
                    mvis::params::MusicInfoAnchor::BottomLeft => "Bottom Left",
                    mvis::params::MusicInfoAnchor::BottomRight => "Bottom Right",
                };
                egui::ComboBox::from_id_salt("music_info_anchor")
                    .selected_text(current_anchor_str)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.music_info_anchor, mvis::params::MusicInfoAnchor::TopLeft, "Top Left");
                        ui.selectable_value(&mut params.music_info_anchor, mvis::params::MusicInfoAnchor::TopRight, "Top Right");
                        ui.selectable_value(&mut params.music_info_anchor, mvis::params::MusicInfoAnchor::BottomLeft, "Bottom Left");
                        ui.selectable_value(&mut params.music_info_anchor, mvis::params::MusicInfoAnchor::BottomRight, "Bottom Right");
                    });
                ui.end_row();

                ui.label("");
                ui.label("Show Spectrum");
                ui.label("");
                ui.checkbox(&mut params.show_mvis_spectrum, "");
                ui.end_row();

                ui.label("");
                ui.label("Repeat Count");
                ui.label("");
                ui.add(egui::Slider::new(&mut params.mvis_repeat_count, 1..=10));
                ui.end_row();

                ui.label("");
                ui.label("Band Count");
                ui.label("");
                ui.add(egui::Slider::new(&mut params.mvis_band_count, 1..=128));
                ui.end_row();


                for param in mvis::params::FloatParam::all() {
                    let meta = param.meta();
                    if meta.category == mvis::params::ParamCategory::Visuals || meta.category == mvis::params::ParamCategory::Visualizer || meta.category == mvis::params::ParamCategory::System {
                        param_row_ui(ui, params, *param);
                    }
                }
            });
    });

    ui.collapsing("Colors", |ui| {
        for i in 0..params.colors.len() {
            ui.horizontal(|ui| {
                ui.label(format!("Color {}", i));
                let srgba = params.colors[i].to_srgba();
                let mut egui_color = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                if ui.color_edit_button_rgba_unmultiplied(&mut egui_color).changed() {
                    params.colors[i] = Color::srgba(egui_color[0], egui_color[1], egui_color[2], egui_color[3]);
                }
            });
        }
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

// TODO: Factor complex query into a type definition
#[allow(clippy::type_complexity)]

fn update_audio_stream(mut stream: ResMut<audio_analysis::AudioStreamReceiver>) {
    while let Ok(bands) = stream.receiver.try_recv() {
        stream.current_bands = bands;
    }
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
    
    // Scale the record radius based on window size so it doesn't get too large
    let min_dimension = params.region_size.x.min(params.region_size.y);
    let max_radius = min_dimension * 0.5;
    let scale = params.record_radius.clamp(0.0, 1.0) * max_radius;
    
    let pos = Vec2::ZERO;

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
    params: Res<SimulationParams>,
    stream: Option<Res<audio_analysis::AudioStreamReceiver>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Sprite), (With<MvisBar>, Without<MvisBarCap>)>,
    mut cap_query: Query<(Entity, &mut Transform, &mut Sprite), (With<MvisBarCap>, Without<MvisBar>)>,
    time: Res<Time>,
    cap_texture: Res<BarCapTexture>,
) {
    if !params.show_mvis_spectrum {
        for (entity, _, _) in query.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _, _) in cap_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Some(stream) = stream else { return };
    let raw_spectrum = &stream.current_bands.spectrum; // [f32; 128]
    let num_bands = params.mvis_band_count.min(128).max(1);
    
    // Apply spatial smoothing (neighbor pull)
    let pull = params.mvis_spatial_smoothing.clamp(0.0, 0.99);
    let mut spectrum = *raw_spectrum;
    for _ in 0..3 { // 3 passes for a wider Gaussian-like blur
        let mut temp = spectrum;
        for b in 0..128 {
            let left = if b > 0 { spectrum[b - 1] } else { spectrum[0] };
            let right = if b < 127 { spectrum[b + 1] } else { spectrum[127] };
            temp[b] = spectrum[b] * (1.0 - pull) + (left + right) * (pull / 2.0);
        }
        spectrum = temp;
    }
    
    // We can repeat the spectrum N times to wrap around the circle symmetrically
    let repeats = params.mvis_repeat_count.max(1);
    let total_bars = num_bands * repeats;

    let win_width = params.region_size.x;
    let win_height = params.region_size.y;
    
    // Scale the record radius based on window size so it doesn't get too large
    let min_dimension = win_width.min(win_height);
    let max_radius = min_dimension * 0.5;
    let base_radius = params.record_radius.clamp(0.0, 1.0) * max_radius;
    
    let max_height = min_dimension * 0.5;
    let spectrum_height = params.mvis_spectrum_height.clamp(0.0, 1.0) * max_height;
    
    let max_thickness = min_dimension * 0.1;
    let bar_thickness = params.mvis_bar_thickness.clamp(0.0, 1.0) * max_thickness;

    let mut existing_bars = query.iter_mut().collect::<Vec<_>>();
    existing_bars.sort_by_key(|(e, _, _)| *e); // Ensure predictable ordering if needed
    
    let mut existing_caps = cap_query.iter_mut().collect::<Vec<_>>();
    existing_caps.sort_by_key(|(e, _, _)| *e);

    let colors = &params.colors;
    let spin = time.elapsed_secs() * params.record_rotation_speed;
    let smooth_amount = params.mvis_spectrum_smoothing.clamp(0.0, 0.99);
    let lerp_factor = if smooth_amount < 0.01 {
        1.0
    } else {
        ((1.0 - smooth_amount) * time.delta_secs() * 30.0).min(1.0)
    };
    let mut i = 0;

    for _r in 0..repeats {
        for b in 0..num_bands {
            // value 0.0 to 1.0 roughly
            let val = spectrum[b].max(0.01);
            let target_height = val * spectrum_height;

            let height = if i < existing_bars.len() {
                let prev_height = existing_bars[i].1.scale.y;
                prev_height + (target_height - prev_height) * lerp_factor
            } else {
                target_height
            };

            let color = colors[b % colors.len()];

            let angle = (i as f32 / total_bars as f32) * std::f32::consts::TAU + spin;

            let mut transform = Transform::default();
            let mut cap_transform = Transform::default();
            
            match params.bar_layout {
                BarLayout::Circular => {
                    // Position along the circle
                    let dir = Vec2::new(angle.cos(), angle.sin());
                    let pos = dir * (base_radius + height * 0.5);
                    transform.translation = pos.extend(0.0);
                    // -PI/2 so the bar points outwards instead of tangentially
                    transform.rotation = Quat::from_rotation_z(angle - std::f32::consts::PI / 2.0);
                    transform.scale = Vec3::new(bar_thickness, height, 1.0);
                    
                    let cap_pos = dir * (base_radius + height);
                    cap_transform.translation = cap_pos.extend(0.0);
                    cap_transform.scale = Vec3::new(bar_thickness, bar_thickness, 1.0);
                }
                BarLayout::Bottom => {
                    let step = win_width / total_bars as f32;
                    let x = (i as f32 + 0.5) * step - (win_width * 0.5);
                    let y = -(win_height * 0.5) + (height * 0.5);
                    transform.translation = Vec3::new(x, y, 0.0);
                    transform.scale = Vec3::new(bar_thickness, height, 1.0);
                    
                    cap_transform.translation = Vec3::new(x, -(win_height * 0.5) + height, 0.0);
                    cap_transform.scale = Vec3::new(bar_thickness, bar_thickness, 1.0);
                }
                BarLayout::Top => {
                    let step = win_width / total_bars as f32;
                    let x = (i as f32 + 0.5) * step - (win_width * 0.5);
                    let y = (win_height * 0.5) - (height * 0.5);
                    transform.translation = Vec3::new(x, y, 0.0);
                    transform.scale = Vec3::new(bar_thickness, height, 1.0);
                    
                    cap_transform.translation = Vec3::new(x, (win_height * 0.5) - height, 0.0);
                    cap_transform.scale = Vec3::new(bar_thickness, bar_thickness, 1.0);
                }
            }

            if i < existing_bars.len() {
                let (_, ref mut t, ref mut s) = existing_bars[i];
                t.translation = transform.translation;
                t.rotation = transform.rotation;
                t.scale = transform.scale;
                s.color = color;
            } else {
                commands.spawn((
                    Sprite {
                        color,
                        ..default()
                    },
                    transform,
                    MvisBar(i),
                ));
            }
            
            if i < existing_caps.len() {
                let (_, ref mut t, ref mut s) = existing_caps[i];
                t.translation = cap_transform.translation;
                t.rotation = cap_transform.rotation;
                t.scale = cap_transform.scale;
                s.color = color;
                s.custom_size = Some(Vec2::new(1.0, 1.0));
            } else {
                commands.spawn((
                    Sprite {
                        color,
                        image: cap_texture.0.clone(),
                        custom_size: Some(Vec2::new(1.0, 1.0)),
                        ..default()
                    },
                    cap_transform,
                    MvisBarCap(i),
                ));
            }
            
            i += 1;
        }
    }

    // Clean up any extra bars if parameters changed (e.g., fewer repeats)
    for j in i..existing_bars.len() {
        commands.entity(existing_bars[j].0).despawn();
    }
    for j in i..existing_caps.len() {
        commands.entity(existing_caps[j].0).despawn();
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

    // Generate circle texture for rounded tops
    let size = 32;
    let mut data = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let cx = 15.5;
            let cy = 15.5;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = if dist <= 15.5 {
                let a = (15.5 - dist).clamp(0.0, 1.0);
                (a * 255.0) as u8
            } else {
                0
            };
            let idx = (y * size + x) * 4;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = alpha;
        }
    }
    let cap_img = Image::new(
        bevy::render::render_resource::Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default()
    );
    let cap_handle = images.add(cap_img);
    commands.insert_resource(BarCapTexture(cap_handle));

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
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Visibility::Hidden,
        RecordVinyl,
    ));

    // Spawn the sticker (smaller circle, initially white so album art texture is visible)
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(1.0, 1.0, 1.0, 1.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.1)),
        Visibility::Hidden,
        RecordSticker,
    ));
}




fn update_window_bounds(
    mut params: ResMut<SimulationParams>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &Transform)>,
) {
    if let Some((camera, camera_transform)) = camera_query.iter().next() {
        let scale = camera_transform.scale;
        
        let (mut w, mut h) = (0.0, 0.0);
        if let Some(window) = window_query.iter().next() {
            w = window.width();
            h = window.height();
        } else if let Some(logical_size) = camera.logical_viewport_size() {
            w = logical_size.x;
            h = logical_size.y;
        }

        if w > 0.0 && h > 0.0 {
            params.region_size = Vec2::new(w * scale.x, h * scale.y);
        }
    }
}

fn resize_background(
    mut query: Query<(&mut Sprite, &BackgroundSprite)>,
    camera_query: Query<&Transform, With<Camera>>,
    params: Res<SimulationParams>,
) {
    if let Some(camera_transform) = camera_query.iter().next() {
        let w = params.region_size.x / camera_transform.scale.x;
        let h = params.region_size.y / camera_transform.scale.y;

        for (mut sprite, bg) in &mut query {
                let target_size = match params.wallpaper_mode {
                    mvis::params::WallpaperMode::Zoom => {
                        let scale_x = (w * camera_transform.scale.x) / bg.image_size.x;
                        let scale_y = (h * camera_transform.scale.y) / bg.image_size.y;
                        let scale = scale_x.max(scale_y);
                        bg.image_size * scale
                    }
                    mvis::params::WallpaperMode::Fit => {
                        let scale_x = (w * camera_transform.scale.x) / bg.image_size.x;
                        let scale_y = (h * camera_transform.scale.y) / bg.image_size.y;
                        let scale = scale_x.min(scale_y);
                        bg.image_size * scale
                    }
                    mvis::params::WallpaperMode::Stretch => {
                        Vec2::new(w * camera_transform.scale.x, h * camera_transform.scale.y)
                    }
                    mvis::params::WallpaperMode::Center => {
                        bg.image_size
                    }
                };

                sprite.custom_size = Some(target_size);
            }
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
