use clap::{Parser, Subcommand};
use mvis::params::{AnimateSource, GravityWellPattern};
use rand::Rng;
use std::fs;
use std::path::PathBuf;

fn get_config_dir() -> PathBuf {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.is_empty() {
            return PathBuf::from(xdg_config_home).join("mvis");
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".config").join("mvis");
        }
    }

    PathBuf::from(".")
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set any simulation parameter dynamically
    Set {
        /// Parameter key (e.g. attraction_strength)
        key: String,
        /// Value to set it to
        value: String,
    },
    /// Lock a parameter from being randomized
    Lock { key: String },
    /// Unlock a parameter to allow it to be randomized again
    Unlock { key: String },
    /// Randomize all unlocked simulation parameters
    Randomize,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Set { key, value } => {
            let config_dir = get_config_dir();
            let config_path = config_dir.join("config.toml");

            let content = match fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read config.toml: {}", e);
                    std::process::exit(1);
                }
            };

            let mut table: toml::Table = match toml::from_str(&content) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to parse config.toml as TOML table: {}", e);
                    std::process::exit(1);
                }
            };

            let mut updated = false;

            if let Some(sim) = table.get_mut("simulation").and_then(|v| v.as_table_mut()) {
                if let Some(existing) = sim.get(&key) {
                    let new_val = match existing {
                        toml::Value::Float(_) => match value.parse::<f64>() {
                            Ok(f) => Some(toml::Value::Float(f)),
                            Err(_) => {
                                eprintln!("Error: '{}' expects a Float value.", key);
                                None
                            }
                        },
                        toml::Value::Integer(_) => match value.parse::<i64>() {
                            Ok(i) => Some(toml::Value::Integer(i)),
                            Err(_) => {
                                eprintln!("Error: '{}' expects an Integer value.", key);
                                None
                            }
                        },
                        toml::Value::Boolean(_) => match value.parse::<bool>() {
                            Ok(b) => Some(toml::Value::Boolean(b)),
                            Err(_) => {
                                eprintln!("Error: '{}' expects a Boolean value (true/false).", key);
                                None
                            }
                        },
                        toml::Value::String(_) => Some(toml::Value::String(value.clone())),
                        _ => {
                            eprintln!("Error: '{}' is a complex type (array/table) which is currently not supported via CLI.", key);
                            None
                        }
                    };

                    if let Some(v) = new_val {
                        sim.insert(key.clone(), v);
                        updated = true;
                    }
                } else {
                    eprintln!("Error: Parameter '{}' not found in config.toml.", key);
                }
            } else {
                eprintln!("Error: [simulation] section not found in config.toml.");
            }

            if updated {
                // Verify it still deserializes correctly into AppConfig!
                let modified_toml = toml::to_string_pretty(&table).unwrap();
                if let Err(e) = toml::from_str::<mvis::config::AppConfig>(&modified_toml) {
                    eprintln!("Error: Applying '{}={}' created an invalid config. It was rejected by the configuration schema.\nDetails: {}", key, value, e);
                    std::process::exit(1);
                }

                if let Err(e) = fs::write(&config_path, modified_toml) {
                    eprintln!("Failed to write updated config: {}", e);
                } else {
                    println!("Successfully updated '{}' to '{}'.", key, value);
                }
            } else {
                std::process::exit(1);
            }
        }
        Commands::Lock { key } => {
            let mut app_config = mvis::config::AppConfig::load_or_create();
            if !app_config.simulation.locked_parameters.contains(&key) {
                app_config.simulation.locked_parameters.push(key.clone());
                app_config.save();
                println!("Locked parameter: {}", key);
            } else {
                println!("Parameter '{}' is already locked.", key);
            }
        }
        Commands::Unlock { key } => {
            let mut app_config = mvis::config::AppConfig::load_or_create();
            if let Some(pos) = app_config
                .simulation
                .locked_parameters
                .iter()
                .position(|x| x == &key)
            {
                app_config.simulation.locked_parameters.remove(pos);
                app_config.save();
                println!("Unlocked parameter: {}", key);
            } else {
                println!("Parameter '{}' is not locked.", key);
            }
        }
        Commands::Randomize => {
            let mut app_config = mvis::config::AppConfig::load_or_create();
            let mut rng = rand::thread_rng();

            macro_rules! randomize {
                ($field:ident, $range:expr) => {
                    if !app_config
                        .simulation
                        .locked_parameters
                        .contains(&stringify!($field).to_string())
                    {
                        app_config.simulation.$field = rng.gen_range($range);
                    }
                };
            }
            macro_rules! randomize_choice {
                ($field:ident, $choices:expr) => {
                    if !app_config
                        .simulation
                        .locked_parameters
                        .contains(&stringify!($field).to_string())
                    {
                        let idx = rng.gen_range(0..$choices.len());
                        app_config.simulation.$field = $choices[idx];
                    }
                };
            }

            randomize!(attraction_strength, 10.0..100.0);
            randomize!(min_dist, 10.0..80.0);
            randomize!(interaction_radius, 50.0..250.0);
            randomize!(density_limit, 0.2..3.0);
            randomize!(dampening, 0.85..0.98);
            randomize!(global_gravity, 0.0..0.05);
            randomize!(emission_intensity, 0.5..3.0);

            let sources = [
                AnimateSource::Off,
                AnimateSource::Sine,
                AnimateSource::Square,
                AnimateSource::Triangle,
                AnimateSource::Sawtooth,
                AnimateSource::SubBass,
                AnimateSource::Bass,
                AnimateSource::LowMid,
                AnimateSource::Mid,
                AnimateSource::HighMid,
                AnimateSource::High,
                AnimateSource::Air,
            ];

            randomize_choice!(animate_attraction, sources);
            randomize_choice!(animate_min_dist, sources);
            randomize_choice!(animate_interaction_radius, sources);
            randomize_choice!(animate_density_limit, sources);
            randomize_choice!(animate_dampening, sources);
            randomize_choice!(animate_global_gravity, sources);
            randomize_choice!(animate_time_scale, sources);
            randomize_choice!(animate_animation_speed, sources);
            randomize_choice!(animate_gravity_well_rotation, sources);
            randomize_choice!(animate_gravity_well_distance_power, sources);
            randomize_choice!(animate_gravity_well_radius, sources);
            randomize_choice!(animate_emission_intensity, sources);
            randomize_choice!(animate_record_radius, sources);
            randomize_choice!(animate_record_rotation_speed, sources);
            randomize_choice!(animate_mvis_spectrum_height, sources);
            randomize_choice!(animate_mvis_bar_thickness, sources);

            randomize!(gravity_wells, 1..10);
            randomize!(gravity_well_radius, 100.0..1000.0);
            randomize_choice!(gravity_center_well, [true, false]);

            let patterns = [
                GravityWellPattern::None,
                GravityWellPattern::Ring,
                GravityWellPattern::Grid,
                GravityWellPattern::Line,
                GravityWellPattern::Spiral,
                GravityWellPattern::Star,
                GravityWellPattern::Cross,
                GravityWellPattern::Random,
            ];
            randomize_choice!(gravity_well_pattern, patterns);

            app_config.simulation.spawn_seed = app_config.simulation.spawn_seed.wrapping_add(1);

            app_config.save();
            println!("Randomized unlocked parameters!");
        }
    }
}
