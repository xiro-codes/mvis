use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BarLayout {
    Circular,
    Top,
    Bottom,
}

impl BarLayout {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Circular => "Circular",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
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
    Visualizer,
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
            }

            impl Default for GeneratedParams {
                fn default() -> Self {
                    Self {
                        $( $field: $default, )*
                    }
                }
            }

            #[derive(clap::Args, Debug, Clone)]
            pub struct GeneratedCliArgs {
                $(
                    #[arg(long, help = $cli_help)]
                    pub $field: Option<f32>,
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
            }
            
            impl GeneratedParams {
                pub fn merge_cli(&mut self, cli: &GeneratedCliArgs) {
                    $(
                        if let Some(val) = cli.$field {
                            self.$field = val;
                        }
                    )*
                }
            }
        }
    }
}

define_simulation_params! {
    mvis_spectrum_height {
        name: "Spectrum Height",
        default: 0.75,
        slider_range: 0.0..=1.0,
        category: ParamCategory::Visuals,
        cli_help: "Base height of spectrum bars"
    },
    mvis_bar_thickness {
        name: "Bar Thickness",
        default: 0.25,
        slider_range: 0.0..=1.0,
        category: ParamCategory::Visuals,
        cli_help: "Thickness of spectrum bars"
    },
    mvis_spectrum_smoothing {
        name: "Spectrum Smoothing",
        default: 0.61875,
        slider_range: 0.0..=0.99,
        category: ParamCategory::Visuals,
        cli_help: "Amount of temporal smoothing applied to the spectrum"
    },
    mvis_spatial_smoothing {
        name: "Neighbor Pull",
        default: 0.495,
        slider_range: 0.0..=0.99,
        category: ParamCategory::Visuals,
        cli_help: "Amount of spatial smoothing across neighbors"
    },
    record_radius {
        name: "Record Radius",
        default: 1.0,
        slider_range: 0.0..=1.0,
        category: ParamCategory::Visuals,
        cli_help: "Radius of the vinyl record"
    },
    record_rotation_speed {
        name: "Record Rotation Speed",
        default: 1.0,
        slider_range: -25.0..=25.0,
        category: ParamCategory::Visuals,
        cli_help: "Rotation speed of the vinyl record"
    }
}

#[derive(Resource, Clone, ExtractResource, Serialize, Deserialize)]
pub struct SimulationParams {
    #[serde(flatten)]
    pub generated: GeneratedParams,

    pub colors: [Color; 10],
    pub region_size: Vec2,
    pub audio_reactivity_power: f32,
    pub bar_layout: BarLayout,
    pub show_mvis_spectrum: bool,
    pub mvis_repeat_count: usize,
    pub mvis_band_count: usize,
    pub disable_wallpaper_colors: bool,
    pub mouse_pos: Vec2,
    pub target_mouse_pos: Vec2,
    pub follow_mouse: bool,
    pub record_exclusion_zone: bool,
    pub show_ui_menu: bool,
    pub music_info_anchor: MusicInfoAnchor,
    pub music_info_padding: Vec2,
}

impl Default for SimulationParams {
    fn default() -> Self {
        let colors = [Color::srgba(0.0, 0.0, 0.0, 0.0); 10];
        
        let generated = GeneratedParams::default();

        Self {
            generated,
            colors,
            region_size: Vec2::new(849.0, 505.0),
            audio_reactivity_power: 0.0,
            bar_layout: BarLayout::Circular,
            show_mvis_spectrum: false,
            mvis_repeat_count: 3,
            mvis_band_count: 32,
            disable_wallpaper_colors: true,
            mouse_pos: Vec2::ZERO,
            target_mouse_pos: Vec2::ZERO,
            follow_mouse: false,
            record_exclusion_zone: false,
            show_ui_menu: true,
            music_info_anchor: MusicInfoAnchor::BottomLeft,
            music_info_padding: Vec2::ZERO,
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
