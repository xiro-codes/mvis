use clap::{CommandFactory, Parser, Subcommand};
use mvis::params::BarLayout;
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
    /// Randomize all simulation parameters
    Randomize,
    /// List all simulation parameters
    List,
    /// Get the current value of a parameter
    Get { key: String },
    /// Generate shell completions
    GenerateCompletion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Manage parameter presets
    Preset {
        #[command(subcommand)]
        action: PresetAction,
    },
}

#[derive(Subcommand)]
enum PresetAction {
    /// List available presets
    List,
    /// Save current configuration as a preset
    Save { name: String },
    /// Load a preset into the active configuration
    Load { name: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateCompletion { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
        Commands::List => {
            let app_config = mvis::config::AppConfig::load_or_create();
            println!("--- Float Parameters ---");
            for param in mvis::params::FloatParam::all() {
                let meta = param.meta();
                let val = param.get_val(&app_config.simulation);
                println!("{}: {} (Range: {:.2}..{:.2})", 
                         meta.id, val, meta.slider_range.start(), meta.slider_range.end());
            }
            println!("--- Other Parameters ---");
            println!("bar_layout: {:?}", app_config.simulation.bar_layout);
        }
        Commands::Get { key } => {
            let app_config = mvis::config::AppConfig::load_or_create();
            
            if let Some(param) = mvis::params::FloatParam::all().into_iter().find(|p| p.meta().id == key) {
                println!("{}", param.get_val(&app_config.simulation));
                return;
            }
            
            match key.as_str() {
                "bar_layout" => println!("{:?}", app_config.simulation.bar_layout),
                _ => {
                    eprintln!("Error: Parameter '{}' not found.", key);
                    std::process::exit(1);
                }
            }
        }
        Commands::Preset { action } => match action {
            PresetAction::List => {
                let presets = mvis::config::AppConfig::list_presets();
                if presets.is_empty() {
                    println!("No presets found. Save one using 'mvis preset save <name>'.");
                } else {
                    println!("Available presets:");
                    for preset in presets {
                        println!("  - {}", preset);
                    }
                }
            }
            PresetAction::Save { name } => {
                let app_config = mvis::config::AppConfig::load_or_create();
                app_config.save_preset(&name);
                println!("Saved current configuration as preset '{}'.", name);
            }
            PresetAction::Load { name } => {
                if let Some(loaded_config) = mvis::config::AppConfig::load_preset(&name) {
                    loaded_config.save();
                    println!("Loaded preset '{}' as the active configuration.", name);
                } else {
                    eprintln!("Error: Preset '{}' not found.", name);
                    std::process::exit(1);
                }
            }
        },
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
        Commands::Randomize => {
            let mut app_config = mvis::config::AppConfig::load_or_create();
            let mut rng = rand::thread_rng();

            macro_rules! randomize_choice {
                ($field:ident, $choices:expr) => {
                    let idx = rng.gen_range(0..$choices.len());
                    app_config.simulation.$field = $choices[idx];
                };
            }

            for param in mvis::params::FloatParam::all() {
                let meta = param.meta();
                let val = rng.gen_range(meta.slider_range.clone());
                param.set_val(&mut app_config.simulation, val);
            }

            let layouts = [
                BarLayout::Circular,
                BarLayout::Top,
                BarLayout::Bottom,
            ];
            randomize_choice!(bar_layout, layouts);

            app_config.save();
            println!("Randomized parameters!");
        }
    }
}
