# Particle Life Wallpaper Configuration Reference

The `mvis-cli` utility allows you to dynamically update the underlying parameters of the wallpaper simulation on the fly. 
Below is a complete reference of all the supported keys and example commands for modifying them. 

All commands are structured as:
```bash
./target/release/mvis-cli set <KEY> <VALUE>
```

> **Note**: Complex types (like Vectors, Arrays, Matrices, and specific layout paddings) cannot be updated via the generic CLI command and must be edited manually in your `~/.config/mvis/config.toml` file.

---

## 🎛️ Simulation Core Settings
These parameters control the fundamental physics and rule-sets of the particle environment.

| Parameter Key | Type | Description | Example CLI Command |
|--------------|------|-------------|----------------------|
| `particle_count` | Integer | Total number of particles in the simulation (Requires restart). | `mvis-cli set particle_count 25000` |
| `particle_types` | Integer | The number of distinct particle groups/colors. | `mvis-cli set particle_types 6` |
| `attraction_strength` | Float | Global multiplier for how strongly particles pull or push each other. | `mvis-cli set attraction_strength 60.5` |
| `time_scale` | Float | Base speed/velocity multiplier of the simulation. | `mvis-cli set time_scale 0.05` |
| `min_dist` | Float | Minimum distance particles try to maintain before repelling. | `mvis-cli set min_dist 20.0` |
| `interaction_radius` | Float | Maximum distance particles can "see" each other. | `mvis-cli set interaction_radius 200.0` |
| `dampening` | Float | Friction/drag applied to particles (1.0 = no friction). | `mvis-cli set dampening 0.9` |
| `infinite_space` | Boolean | Whether particles wrap around the screen edges. | `mvis-cli set infinite_space true` |
| `global_gravity` | Float | A constant force pulling particles to the center (negative pushes outwards). | `mvis-cli set global_gravity 0.0` |
| `scale` | Float | Visual zoom factor of the rendering. | `mvis-cli set scale 2.0` |
| `density_limit` | Float | How tightly particles are allowed to pack together. | `mvis-cli set density_limit 0.5` |
| `spawn_seed` | Integer | Randomization seed for generating the interaction matrix. | `mvis-cli set spawn_seed 42` |
| `continuous_mutation` | Boolean | If true, the interaction matrix gradually drifts over time. | `mvis-cli set continuous_mutation false` |

---

## 🌌 Gravity Well Settings
Gravity wells pull particles into beautiful organized patterns.

| Parameter Key | Type | Description | Example CLI Command |
|--------------|------|-------------|----------------------|
| `gravity_wells` | Integer | Number of active gravity wells. | `mvis-cli set gravity_wells 4` |
| `gravity_well_radius` | Float | How far out the gravity wells are placed from the center. | `mvis-cli set gravity_well_radius 500.0` |
| `gravity_center_well` | Boolean | Enable an extra gravity well in the direct center. | `mvis-cli set gravity_center_well true` |
| `gravity_well_pattern` | Enum | Layout structure. Options: `None`, `Ring`, `Grid`, `Line`, `Spiral`, `Star`, `Cross`, `Random` | `mvis-cli set gravity_well_pattern Ring` |
| `gravity_well_rotation` | Float | Static angle offset for the pattern. | `mvis-cli set gravity_well_rotation 3.14` |
| `gravity_well_rotation_speed` | Float | Constant spinning speed for the wells. | `mvis-cli set gravity_well_rotation_speed 0.5` |
| `gravity_well_distance_power` | Float | Scale gravity strength based on distance from the center. | `mvis-cli set gravity_well_distance_power 0.1` |

---

## 🎵 Audio Reactivity & Animation
These settings link the simulation parameters to the MPD audio analysis buffer (e.g. Bass, Mid, High).
*Valid Enum states for Animate parameters: `Off`, `Sine`, `Square`, `Triangle`, `Sawtooth`, `SubBass`, `Bass`, `LowMid`, `Mid`, `HighMid`, `High`, `Air`*

| Parameter Key | Type | Description | Example CLI Command |
|--------------|------|-------------|----------------------|
| `audio_reactivity_power` | Float | Global multiplier for audio reactivity intensity. | `mvis-cli set audio_reactivity_power 1.5` |
| `animate_attraction` | Enum | Audio band that modulates `attraction_strength`. | `mvis-cli set animate_attraction Bass` |
| `animate_min_dist` | Enum | Audio band that modulates `min_dist`. | `mvis-cli set animate_min_dist Mid` |
| `animate_interaction_radius` | Enum | Audio band that modulates `interaction_radius`. | `mvis-cli set animate_interaction_radius High` |
| `animate_density_limit` | Enum | Audio band that modulates `density_limit`. | `mvis-cli set animate_density_limit Off` |
| `animate_dampening` | Enum | Audio band that modulates `dampening`. | `mvis-cli set animate_dampening Off` |
| `animate_global_gravity` | Enum | Audio band that modulates `global_gravity`. | `mvis-cli set animate_global_gravity SubBass` |
| `animate_time_scale` | Enum | Audio band that modulates the simulation speed. | `mvis-cli set animate_time_scale HighMid` |
| `animate_animation_speed` | Enum | Modulates the speed of programmatic oscillators (Sine, Square). | `mvis-cli set animate_animation_speed Off` |
| `animate_gravity_well_radius` | Enum | Audio band that modulates `gravity_well_radius`. | `mvis-cli set animate_gravity_well_radius Bass` |
| `animate_gravity_well_rotation` | Enum | Audio band that modulates `gravity_well_rotation`. | `mvis-cli set animate_gravity_well_rotation Mid` |
| `animate_gravity_well_distance_power`| Enum | Audio band that modulates `gravity_well_distance_power`. | `mvis-cli set animate_gravity_well_distance_power High` |

---

## 📀 Record Visualizer & Spectrum UI
Settings for the center "Vinyl Record" and audio spectrum bars.

| Parameter Key | Type | Description | Example CLI Command |
|--------------|------|-------------|----------------------|
| `show_mvis_spectrum` | Boolean | Display the audio spectrum bars around the center. | `mvis-cli set show_mvis_spectrum true` |
| `mvis_spectrum_height` | Float | Max height of the spectrum bars. | `mvis-cli set mvis_spectrum_height 150.0` |
| `mvis_bar_thickness` | Float | Width of the spectrum bars. | `mvis-cli set mvis_bar_thickness 4.0` |
| `mvis_repeat_count` | Integer | Number of times the spectrum is mirrored around the circle. | `mvis-cli set mvis_repeat_count 4` |
| `record_radius` | Float | Radius of the center vinyl record/album art. | `mvis-cli set record_radius 200.0` |
| `record_exclusion_zone` | Boolean | Whether particles are physically repelled from the record surface. | `mvis-cli set record_exclusion_zone true` |
| `record_rotation_speed` | Float | How fast the record visually spins. | `mvis-cli set record_rotation_speed 1.0` |
| `animate_mvis_spectrum_height` | Enum | Audio band that modulates `mvis_spectrum_height`. | `mvis-cli set animate_mvis_spectrum_height Bass` |
| `animate_mvis_bar_thickness` | Enum | Audio band that modulates `mvis_bar_thickness`. | `mvis-cli set animate_mvis_bar_thickness High` |
| `animate_record_radius` | Enum | Audio band that modulates the record's size. | `mvis-cli set animate_record_radius SubBass` |
| `animate_record_rotation_speed` | Enum | Audio band that modulates the record's spinning velocity. | `mvis-cli set animate_record_rotation_speed Mid` |

---

## 🎨 Miscellaneous & Theming
Toggles and utility configurations.

| Parameter Key | Type | Description | Example CLI Command |
|--------------|------|-------------|----------------------|
| `music_info_anchor` | Enum | Where the track info is displayed. Options: `TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`. | `mvis-cli set music_info_anchor BottomRight` |
| `emission_intensity` | Float | Brightness multiplier for the particle renderer. | `mvis-cli set emission_intensity 1.5` |
| `animate_emission_intensity` | Enum | Audio band that modulates `emission_intensity`. | `mvis-cli set animate_emission_intensity Bass` |
| `disable_wallpaper_colors` | Boolean | If true, particles will NOT adapt their colors to the album art/wallpaper. | `mvis-cli set disable_wallpaper_colors true` |
| `show_debug_visuals` | Boolean | Displays outlines indicating where gravity wells are located. | `mvis-cli set show_debug_visuals false` |
| `show_ui_menu` | Boolean | Toggles the Egui configuration dashboard. | `mvis-cli set show_ui_menu false` |
| `matrix_base` | Float | Base average value generated into the interaction matrix. | `mvis-cli set matrix_base 0.0` |
| `matrix_spread` | Float | Amplitude/spread of random values in the interaction matrix. | `mvis-cli set matrix_spread 1.5` |
| `auto_camera` | Boolean | If true, camera slowly pans/zooms automatically. | `mvis-cli set auto_camera false` |
| `follow_mouse` | Boolean | If true, global gravity pulls towards your mouse cursor. | `mvis-cli set follow_mouse true` |
