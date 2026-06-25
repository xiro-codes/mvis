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
            pub struct GeneratedParams {
                $( pub $field: f32, )*
                $( pub [<animate_ $field>]: crate::params::AnimateSource, )*
            }

            impl Default for GeneratedParams {
                fn default() -> Self {
                    Self {
                        $( $field: $default, )*
                        $( [<animate_ $field>]: $anim_default, )*
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
            }
            
            impl GeneratedParams {
                pub fn merge_cli(&mut self, cli: &GeneratedCliArgs) {
                    $(
                        if let Some(val) = cli.$field {
                            self.$field = val;
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
            Color::srgba(0.0, 0.0, 0.003921568859368563, 1.0),
            Color::srgba(0.3450980484485626, 0.3176470696926117, 0.027450980618596077, 1.0),
            Color::srgba(0.8313725590705872, 0.7058823704719543, 0.10588235408067703, 1.0),
            Color::srgba(0.18039216101169586, 0.062745101749897, 0.29411765933036804, 1.0),
            Color::srgba(0.5529412031173706, 0.4745098054409027, 0.1764705926179886, 1.0),
            Color::srgba(0.33725491166114807, 0.2705882489681244, 0.4941176474094391, 1.0),
            Color::srgba(0.6078431606292725, 0.5411764979362488, 0.7764706015586853, 1.0),
            Color::srgba(0.7960784435272217, 0.7607843279838562, 0.8745098114013672, 1.0),
            Color::srgba(0.4745098054409027, 0.4313725531101227, 0.5960784554481506, 1.0),
            Color::srgba(0.10588235408067703, 0.10588235408067703, 0.10980392247438431, 1.0),
        ];

        let interaction_matrix = [
            [ 0.36746087670326233, 0.5923157930374146, 0.19599111378192902, -0.0016689160838723183, -0.1318061649799347, 0.9716295003890991, -0.8917920589447021, -0.10412275791168213, -0.9412205219268799, -0.9220917224884033 ],
            [ 0.03154454752802849, 0.22886747121810913, -1.258409023284912, 0.4010147750377655, 0.534250795841217, 0.08655649423599243, -0.2068861722946167, -0.951228141784668, 0.06904327869415283, -0.5543262362480164 ],
            [ -0.34359216690063477, 0.14847570657730103, 0.12894225120544434, 0.7316543459892273, 0.5942926406860352, 0.26411014795303345, -0.2729946970939636, 0.8046836853027344, 0.5259115695953369, -0.7210686206817627 ],
            [ 0.8169270157814026, -0.33928602933883667, -0.5783749222755432, -0.5593540668487549, -0.3831753730773926, -0.46698787808418274, 0.18511545658111572, -0.31009453535079956, -0.08018958568572998, 0.2572082281112671 ],
            [ -0.14619778096675873, -0.680194616317749, 0.429594486951828, -0.5018467903137207, 0.1693573147058487, -1.0741091966629028, 0.329922080039978, 0.1684778928756714, 0.6303567886352539, -0.10405576229095459 ],
            [ 0.9689998030662537, 0.9269834756851196, 0.24940870702266693, -0.6748055815696716, 0.22176654636859894, -0.8136312961578369, 0.8680984973907471, -0.5022386908531189, -0.7711994647979736, 0.8285119533538818 ],
            [ -0.422264039516449, -0.8836026191711426, -0.8731560707092285, 0.8061394691467285, 0.2592970132827759, 0.5276455879211426, -0.12442886829376221, -0.8750081062316895, -0.7789309024810791, 0.906092643737793 ],
            [ -0.12043964862823486, -0.11559760570526123, -0.18545424938201904, 0.07371556758880615, -0.510011613368988, 0.5816385746002197, 0.2186356782913208, -0.5109104514122009, -0.3559498190879822, -0.2350989580154419 ],
            [ 0.9820497035980225, -0.6847898960113525, 0.17075049877166748, -0.77427077293396, 0.15587151050567627, 0.5507144927978516, 0.8404436111450195, -0.7104218006134033, -0.17552459239959717, 0.30497705936431885 ],
            [ 0.5557210445404053, -0.9692506790161133, 0.2614020109176636, 0.6928122043609619, 0.9845166206359863, -0.8143484592437744, 0.1198500394821167, 0.6430087089538574, -0.5416387915611267, -0.9148659706115723 ]
        ];

        let type_proportions = [
            0.2938831150531769, 1.8068894147872925, 0.31394582986831665, 1.2866599559783936, 1.1281758546829224, 
            0.3120129108428955, 0.3437986671924591, 0.6369897127151489, 1.1660585403442383, 0.20540815591812134
        ];

        let locked_parameters = vec![
            "gravity_wells".to_string(),
            "gravity_well_radius".to_string(),
            "gravity_well_rotation_speed".to_string(),
            "animate_gravity_well_rotation".to_string(),
            "animate_gravity_well_radius".to_string(),
            "gravity_well_distance_power".to_string(),
            "record_exclusion_zone".to_string(),
            "record_radius".to_string(),
            "animate_record_radius".to_string(),
            "record_rotation_speed".to_string(),
            "animate_record_rotation_speed".to_string(),
            "show_mvis_spectrum".to_string(),
            "mvis_spectrum_height".to_string(),
            "mvis_bar_thickness".to_string(),
            "mvis_repeat_count".to_string(),
            "animate_mvis_bar_thickness".to_string(),
            "animate_mvis_spectrum_height".to_string(),
            "show_debug_visuals".to_string(),
            "gravity_well_pattern".to_string(),
        ];

        Self {
            generated: GeneratedParams::default(),
            particle_count: 15000,
            particle_types: 6,
            region_size: Vec2::new(2560.0, 1080.0),
            scale: 8.0,
            interaction_matrix,
            colors,
            infinite_space: true,
            type_proportions,
            spawn_seed: 9,
            continuous_mutation: true,
            is_animating_time: false,
            target_time_scale: 0.05000000074505806,
            slider_animation_time: 11168.76171875,
            audio_reactivity_power: 0.25,
            auto_camera: true,
            gravity_wells: 1,
            gravity_center_well: true,
            gravity_well_pattern: GravityWellPattern::Grid,
            gravity_well_rotation: 3079.54541015625,
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
            show_debug_visuals: true,
            record_exclusion_zone: true,
            show_mvis_spectrum: true,
            mvis_repeat_count: 3,
            show_ui_menu: true,
            music_info_anchor: MusicInfoAnchor::BottomLeft,
            music_info_padding: Vec2::new(20.0, 20.0),
            locked_parameters,
            smoothed_parameters: Vec::new(),
            smoothing_strength: 0.8,
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
