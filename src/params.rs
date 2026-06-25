use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AnimateSource {
    Off,
    Sine,
    Square,
    Triangle,
    Sawtooth,
    SubBass,
    Bass,
    LowMid,
    Mid,
    HighMid,
    High,
    Air,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GravityWellPattern {
    None,
    Ring,
    Grid,
    Line,
    Spiral,
    Star,
    Cross,
    Random,
}

impl GravityWellPattern {
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Ring => "Ring",
            Self::Grid => "Grid",
            Self::Line => "Line",
            Self::Spiral => "Spiral",
            Self::Star => "Star",
            Self::Cross => "Cross",
            Self::Random => "Random",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum MusicInfoAnchor {
    TopLeft,
    TopRight,
    #[default]
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ParamCategory {
    Physics,
    GravityWells,
    Visuals,
    System,
}

pub struct ParamMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub slider_range: std::ops::RangeInclusive<f32>,
    pub category: ParamCategory,
}

macro_rules! define_simulation_params {
    (
        $(
            $field:ident {
                name: $name:expr,
                default: $default:expr,
                anim_default: $anim_default:expr,
                slider_range: $slider_range:expr,
                category: $category:expr,
                cli_help: $cli_help:expr
            }
        ),* $(,)?
    ) => {
        paste::paste! {
            #[derive(Clone, serde::Serialize, serde::Deserialize)]
            #[serde(default)]
            pub struct GeneratedParams {
                $( pub $field: f32, )*
                $( pub [<animate_ $field>]: crate::params::AnimateSource, )*
                $( pub [<invert_ $field>]: bool, )*
            }

            impl Default for GeneratedParams {
                fn default() -> Self {
                    Self {
                        $( $field: $default, )*
                        $( [<animate_ $field>]: $anim_default, )*
                        $( [<invert_ $field>]: false, )*
                    }
                }
            }

            #[derive(clap::Args, Debug, Clone)]
            pub struct GeneratedCliArgs {
                $(
                    #[arg(long, help = $cli_help)]
                    pub $field: Option<f32>,
                    
                    #[arg(long, help = concat!("Audio animation source for ", stringify!($name)))]
                    pub [<animate_ $field>]: Option<String>,

                    #[arg(long, help = concat!("Invert audio animation for ", stringify!($name)))]
                    pub [<invert_ $field>]: Option<bool>,
                )*
            }

            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            pub enum FloatParam {
                $( [<$field:camel>] , )*
            }

            impl FloatParam {
                pub fn all() -> &'static [FloatParam] {
                    &[ $( FloatParam::[<$field:camel>] ),* ]
                }

                pub fn meta(&self) -> ParamMeta {
                    match self {
                        $(
                            Self::[<$field:camel>] => ParamMeta {
                                id: stringify!($field),
                                name: $name,
                                slider_range: $slider_range,
                                category: $category,
                            },
                        )*
                    }
                }

                pub fn get_val(&self, p: &crate::params::SimulationParams) -> f32 {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.$field, )*
                    }
                }

                pub fn set_val(&self, p: &mut crate::params::SimulationParams, val: f32) {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.$field = val, )*
                    }
                }
                
                pub fn get_anim_source(&self, p: &crate::params::SimulationParams) -> crate::params::AnimateSource {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.[<animate_ $field>], )*
                    }
                }

                pub fn set_anim_source(&self, p: &mut crate::params::SimulationParams, source: crate::params::AnimateSource) {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.[<animate_ $field>] = source, )*
                    }
                }

                pub fn get_invert(&self, p: &crate::params::SimulationParams) -> bool {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.[<invert_ $field>], )*
                    }
                }

                pub fn set_invert(&self, p: &mut crate::params::SimulationParams, invert: bool) {
                    match self {
                        $( Self::[<$field:camel>] => p.generated.[<invert_ $field>] = invert, )*
                    }
                }
            }
            
            impl GeneratedParams {
                pub fn merge_cli(&mut self, cli: &GeneratedCliArgs) {
                    $(
                        if let Some(val) = cli.$field {
                            self.$field = val;
                        }
                        if let Some(val) = cli.[<invert_ $field>] {
                            self.[<invert_ $field>] = val;
                        }
                        /* Needs parser logic for AnimateSource */
                    )*
                }
            }
        }
    }
}

define_simulation_params! {
    attraction_strength {
        name: "Force Multiplier",
        default: 4.1469326,
        anim_default: crate::params::AnimateSource::LowMid,
        slider_range: -500.0..=500.0,
        category: ParamCategory::Physics,
        cli_help: "Global attraction/repulsion force multiplier"
    },
    dampening {
        name: "Dampening",
        default: 0.9,
        anim_default: crate::params::AnimateSource::LowMid,
        slider_range: 0.0..=1.0,
        category: ParamCategory::Physics,
        cli_help: "Friction applied to particle velocity"
    },
    min_dist {
        name: "Minimum Distance",
        default: 14.654234,
        anim_default: crate::params::AnimateSource::Sawtooth,
        slider_range: 0.0..=500.0,
        category: ParamCategory::Physics,
        cli_help: "Minimum distance before strong repulsion kicks in"
    },
    interaction_radius {
        name: "Interaction Radius",
        default: 77.135315,
        anim_default: crate::params::AnimateSource::Air,
        slider_range: 10.0..=1000.0,
        category: ParamCategory::Physics,
        cli_help: "Radius for particle interactions"
    },
    density_limit {
        name: "Density Limit",
        default: 0.139289,
        anim_default: crate::params::AnimateSource::HighMid,
        slider_range: 0.01..=50.0,
        category: ParamCategory::Physics,
        cli_help: "Soft cap on particle density"
    },
    global_gravity {
        name: "Global Gravity",
        default: 0.0,
        anim_default: crate::params::AnimateSource::Square,
        slider_range: -5.0..=5.0,
        category: ParamCategory::Physics,
        cli_help: "Constant downward/upward force"
    },
    center_gravity {
        name: "Center Gravity",
        default: 0.0,
        anim_default: crate::params::AnimateSource::Off,
        slider_range: -5.0..=5.0,
        category: ParamCategory::Physics,
        cli_help: "Gravitational pull towards the center"
    },
    time_scale {
        name: "Time Scale",
        default: 0.0630308,
        anim_default: crate::params::AnimateSource::SubBass,
        slider_range: -10.0..=10.0,
        category: ParamCategory::System,
        cli_help: "Speed of the simulation physics"
    },
    animation_speed {
        name: "Auto-Animate Speed",
        default: 0.452255,
        anim_default: crate::params::AnimateSource::Air,
        slider_range: 0.0..=20.0,
        category: ParamCategory::System,
        cli_help: "Speed of auto-animation"
    },
    gravity_well_rotation_speed {
        name: "Rotation Speed",
        default: 0.315154,
        anim_default: crate::params::AnimateSource::SubBass,
        slider_range: -20.0..=20.0,
        category: ParamCategory::GravityWells,
        cli_help: "Rotation speed of gravity wells"
    },
    gravity_well_distance_power {
        name: "Distance Power",
        default: 2.5,
        anim_default: crate::params::AnimateSource::Off,
        slider_range: -10.0..=10.0,
        category: ParamCategory::GravityWells,
        cli_help: "Inverse distance power for gravity wells"
    },
    gravity_well_radius {
        name: "Pattern Radius",
        default: 293.084686,
        anim_default: crate::params::AnimateSource::Sawtooth,
        slider_range: 0.0..=5000.0,
        category: ParamCategory::GravityWells,
        cli_help: "Radius of the gravity well pattern"
    },
    emission_intensity {
        name: "Emission Intensity",
        default: 1.0045105,
        anim_default: crate::params::AnimateSource::Air,
        slider_range: 0.0..=50.0,
        category: ParamCategory::Visuals,
        cli_help: "Bloom emission intensity"
    },
    record_radius {
        name: "Record Radius",
        default: 113.03079,
        anim_default: crate::params::AnimateSource::SubBass,
        slider_range: 10.0..=2000.0,
        category: ParamCategory::Visuals,
        cli_help: "Radius of the vinyl record"
    },
    record_rotation_speed {
        name: "Record Rotation Speed",
        default: 1.0,
        anim_default: crate::params::AnimateSource::Off,
        slider_range: -50.0..=50.0,
        category: ParamCategory::Visuals,
        cli_help: "Rotation speed of the vinyl record"
    },
    mvis_spectrum_height {
        name: "Spectrum Height",
        default: 100.0,
        anim_default: crate::params::AnimateSource::Off,
        slider_range: 1.0..=2000.0,
        category: ParamCategory::Visuals,
        cli_help: "Height of the audio spectrum visualizer"
    },
    mvis_bar_thickness {
        name: "Bar Thickness",
        default: 3.0,
        anim_default: crate::params::AnimateSource::Off,
        slider_range: 0.1..=100.0,
        category: ParamCategory::Visuals,
        cli_help: "Thickness of audio spectrum bars"
    }
}

#[derive(Resource, Clone, ExtractResource, Serialize, Deserialize)]
pub struct SimulationParams {
    #[serde(flatten)]
    pub generated: GeneratedParams,

    pub particle_count: usize,
    pub particle_types: usize,
    pub region_size: Vec2,
    pub scale: f32,
    pub interaction_matrix: [[f32; 10]; 10],
    pub colors: [Color; 10],
    pub infinite_space: bool,
    pub type_proportions: [f32; 10],
    pub spawn_seed: u32,
    pub continuous_mutation: bool,
    pub is_animating_time: bool,
    pub target_time_scale: f32,
    pub slider_animation_time: f32,
    pub audio_reactivity_power: f32,
    pub auto_camera: bool,
    pub gravity_wells: u32,
    pub gravity_center_well: bool,
    pub gravity_well_pattern: GravityWellPattern,
    pub gravity_well_rotation: f32,
    pub disable_wallpaper_colors: bool,
    pub lock_rules: bool,
    pub lock_environment: bool,
    pub lock_gravity_wells: bool,
    pub lock_audio_reactivity: bool,
    pub matrix_base: f32,
    pub matrix_spread: f32,
    pub mouse_pos: Vec2,
    pub target_mouse_pos: Vec2,
    pub follow_mouse: bool,
    pub show_debug_visuals: bool,
    pub record_exclusion_zone: bool,
    pub show_mvis_spectrum: bool,
    pub mvis_repeat_count: usize,
    pub show_ui_menu: bool,
    pub music_info_anchor: MusicInfoAnchor,
    pub music_info_padding: Vec2,
    pub locked_parameters: Vec<String>,
    pub smoothed_parameters: Vec<String>,
    pub smoothing_strength: f32,
    #[serde(skip)]
    pub smoothed_audio_energy: f32,
}

impl Default for SimulationParams {
    fn default() -> Self {
        let colors = [
            Color::srgba(0.07450980693101883, 0.04313725605607033, 0.0, 1.0),
            Color::srgba(0.658823549747467, 0.364705890417099, 0.003921568859368563, 1.0),
            Color::srgba(0.9215686321258545, 0.6117647290229797, 0.027450980618596077, 1.0),
            Color::srgba(0.3529411852359772, 0.239215686917305, 0.09803921729326248, 1.0),
            Color::srgba(0.7372549176216125, 0.5529412031173706, 0.30588236451148987, 1.0),
            Color::srgba(0.9333333373069763, 0.7647058963775635, 0.4431372582912445, 1.0),
            Color::srgba(0.9960784316062927, 0.9254902005195618, 0.7843137383460999, 1.0),
            Color::srgba(0.5607843399047852, 0.48235294222831726, 0.1764705926179886, 1.0),
            Color::srgba(0.4156862795352936, 0.3490196168422699, 0.27843138575553894, 1.0),
            Color::srgba(0.95686274766922, 0.8078431487083435, 0.13333334028720856, 1.0),
        ];

        let interaction_matrix = [
            [ -0.4584639072418213, -0.26592040061950684, -0.16838450729846954, 0.40076443552970886, -0.8251513838768005, 0.7145001292228699, 0.2743682265281677, 0.6413748264312744, 0.934380054473877, -0.9984729290008545 ],
            [ -0.02315349504351616, 0.5694538354873657, 0.11004456132650375, -0.5187170505523682, 0.9535415172576904, -1.3105475902557373, 0.9969793558120728, 0.13139666616916656, -0.27138751745224, -0.06906378269195557 ],
            [ 0.35581398010253906, 0.06696033477783203, 0.3552507162094116, 0.012173337861895561, -0.276632159948349, -0.5142939686775208, 0.6891143918037415, -1.1402267217636108, -0.1361008882522583, -0.29954880475997925 ],
            [ -1.0644022226333618, -0.34176263213157654, 0.7832526564598083, 0.8229621052742004, -0.45459744334220886, -0.006619784981012344, -0.9471387267112732, 0.5191933512687683, -0.9528546333312988, -0.5340935587882996 ],
            [ -0.7972061634063721, 1.1859968900680542, -0.9855568408966064, 0.5454855561256409, -0.6463690996170044, -0.47649946808815, -0.05501170828938484, -0.7989727854728699, 0.682081937789917, 0.486641526222229 ],
            [ -0.6018129587173462, -0.02907242253422737, -0.9045098423957825, 0.7574538588523865, 0.5073769688606262, 0.31334030628204346, 0.04360204562544823, -0.5493527054786682, -0.18629086017608643, 0.5658485889434814 ],
            [ -0.008477546274662018, 0.8837034106254578, -0.6514148116111755, -0.4928436875343323, 0.6485894918441772, -0.07401032745838165, 0.5064451694488525, -0.7678699493408203, 0.46241486072540283, 0.8257298469543457 ],
            [ -0.5942475199699402, -0.28762751817703247, 0.9732380509376526, -0.2909482419490814, 1.0253223180770874, -0.5539048314094543, 0.04009367153048515, -0.5375773906707764, 0.5779409408569336, 0.09500014781951904 ],
            [ 0.9588718414306641, 0.5375478267669678, 0.010481715202331543, -0.40414804220199585, -0.6560637950897217, 0.39263737201690674, -0.7480764389038086, -0.01745474338531494, 0.6105186939239502, -0.32213324308395386 ],
            [ -0.8824605941772461, 0.36413490772247314, -0.3417190909385681, -0.7885251045227051, -0.617893636226654, 0.09347450733184814, -0.03511250019073486, 0.2689317464828491, -0.8023805618286133, 0.6290018558502197 ]
        ];

        let type_proportions = [
            0.46610355377197266, 1.6373518705368042, 0.17504636943340302, 1.3087286949157715, 1.1983627080917358, 
            0.35313013195991516, 1.167947769165039, 1.3412078619003296, 1.6417570114135742, 0.5637240409851074
        ];

        let locked_parameters = vec![
            "gravity_wells".to_string(),
            "animate_gravity_well_rotation".to_string(),
            "animate_gravity_well_radius".to_string(),
            "record_exclusion_zone".to_string(),
            "animate_record_radius".to_string(),
            "animate_record_rotation_speed".to_string(),
            "show_mvis_spectrum".to_string(),
            "mvis_spectrum_height".to_string(),
            "mvis_bar_thickness".to_string(),
            "mvis_repeat_count".to_string(),
            "animate_mvis_bar_thickness".to_string(),
            "animate_mvis_spectrum_height".to_string(),
            "show_debug_visuals".to_string(),
            "record_rotation_speed".to_string(),
            "gravity_well_rotation_speed".to_string(),
            "gravity_well_distance_power".to_string(),
            "dampening".to_string(),
        ];
        
        let mut generated = GeneratedParams::default();
        generated.attraction_strength = -162.70590209960938;
        generated.animate_attraction_strength = crate::params::AnimateSource::Square;
        generated.invert_attraction_strength = true;
        
        generated.dampening = 0.09999999403953552;
        generated.animate_dampening = crate::params::AnimateSource::LowMid;
        generated.invert_dampening = false;

        generated.min_dist = 369.6646728515625;
        generated.animate_min_dist = crate::params::AnimateSource::Sine;
        generated.invert_min_dist = true;

        generated.interaction_radius = 846.1050415039063;
        generated.animate_interaction_radius = crate::params::AnimateSource::Mid;
        generated.invert_interaction_radius = true;

        generated.density_limit = 48.33196258544922;
        generated.animate_density_limit = crate::params::AnimateSource::Sawtooth;
        generated.invert_density_limit = true;

        generated.global_gravity = 0.8349452018737793;
        generated.animate_global_gravity = crate::params::AnimateSource::LowMid;
        generated.invert_global_gravity = false;

        generated.center_gravity = -0.6869460344314575;
        generated.animate_center_gravity = crate::params::AnimateSource::HighMid;
        generated.invert_center_gravity = true;

        generated.time_scale = 1.052954912185669;
        generated.animate_time_scale = crate::params::AnimateSource::SubBass;
        generated.invert_time_scale = false;

        generated.animation_speed = 16.891010284423828;
        generated.animate_animation_speed = crate::params::AnimateSource::Mid;
        generated.invert_animation_speed = true;

        generated.gravity_well_rotation_speed = 0.5999984741210938;
        generated.animate_gravity_well_rotation_speed = crate::params::AnimateSource::High;
        generated.invert_gravity_well_rotation_speed = false;

        generated.gravity_well_distance_power = 1.418257474899292;
        generated.animate_gravity_well_distance_power = crate::params::AnimateSource::LowMid;
        generated.invert_gravity_well_distance_power = false;

        generated.gravity_well_radius = 166.8371124267578;
        generated.animate_gravity_well_radius = crate::params::AnimateSource::Sawtooth;
        generated.invert_gravity_well_radius = false;

        generated.emission_intensity = 1.6683710813522339;
        generated.animate_emission_intensity = crate::params::AnimateSource::Sawtooth;
        generated.invert_emission_intensity = false;

        generated.record_radius = 286.9060363769531;
        generated.animate_record_radius = crate::params::AnimateSource::Air;
        generated.invert_record_radius = false;

        generated.record_rotation_speed = 1.0;
        generated.animate_record_rotation_speed = crate::params::AnimateSource::Off;
        generated.invert_record_rotation_speed = false;

        generated.mvis_spectrum_height = 100.0;
        generated.animate_mvis_spectrum_height = crate::params::AnimateSource::Off;
        generated.invert_mvis_spectrum_height = false;

        generated.mvis_bar_thickness = 3.0;
        generated.animate_mvis_bar_thickness = crate::params::AnimateSource::Off;
        generated.invert_mvis_bar_thickness = false;

        Self {
            generated,
            particle_count: 100,
            particle_types: 8,
            region_size: Vec2::new(849.0, 505.0),
            scale: 8.0,
            interaction_matrix,
            colors,
            infinite_space: true,
            type_proportions,
            spawn_seed: 14,
            continuous_mutation: true,
            is_animating_time: false,
            target_time_scale: 0.05000000074505806,
            slider_animation_time: 48085.86328125,
            audio_reactivity_power: 0.25,
            auto_camera: true,
            gravity_wells: 50,
            gravity_center_well: true,
            gravity_well_pattern: GravityWellPattern::Ring,
            gravity_well_rotation: 3887.22265625,
            disable_wallpaper_colors: true,
            lock_rules: false,
            lock_environment: false,
            lock_gravity_wells: false,
            lock_audio_reactivity: false,
            matrix_base: 0.0,
            matrix_spread: 1.0,
            mouse_pos: Vec2::ZERO,
            target_mouse_pos: Vec2::ZERO,
            follow_mouse: false,
            show_debug_visuals: false,
            record_exclusion_zone: true,
            show_mvis_spectrum: true,
            mvis_repeat_count: 3,
            show_ui_menu: true,
            music_info_anchor: MusicInfoAnchor::BottomLeft,
            music_info_padding: Vec2::new(20.0, 20.0),
            locked_parameters,
            smoothed_parameters: vec!["record_radius".to_string()],
            smoothing_strength: 0.800000011920929,
            smoothed_audio_energy: 0.0,
        }
    }
}

impl std::ops::Deref for SimulationParams {
    type Target = GeneratedParams;

    fn deref(&self) -> &Self::Target {
        &self.generated
    }
}

impl std::ops::DerefMut for SimulationParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.generated
    }
}
