# Particle Life Live Wallpaper (MVIS)

An audio-reactive, high-performance live wallpaper and Particle Life simulation built with Rust, Bevy, and compute shaders. The simulation dynamically reacts to music playing via MPD (Music Player Daemon).

## Features

- **Live Wallpaper Mode**: Run the simulation as your desktop background (`mvis-wallpaper`). Automatically extracts colors from your current wallpaper or MPD album art.
- **Advanced Audio Reactivity**: Streams real-time audio analysis from MPD.
- **Dynamic Gravity Matrix**: Multi-gravity well system with customizable attraction patterns (Star, Cross, Random) and inverse-distance calculation.
- **Live CLI Control (`mvis-cli`)**: Adjust simulation parameters, lock/unlock specific settings, and randomize variables on the fly while the daemon runs.
- **XDG-Compliant Configuration**: Cleanly persists settings to `~/.config/mvis/config.toml`.

## Nix / Flake Setup

You can run the application directly from the flake without installing:
```bash
nix run github:xiro-codes/mvis
```

To install the package on a NixOS system, you can include the repository in your flake inputs and add the package via a NixOS module.

Here is a minimal `flake.nix` demonstrating how to pass the inputs and add `mvis` to your system packages:

```nix
{
  description = "A minimal NixOS configuration including mvis";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    mvis.url = "github:xiro-codes/mvis";
  };

  outputs = { self, nixpkgs, mvis, ... }@inputs: {
    nixosConfigurations.myHost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      
      # Pass flake inputs to all your modules
      specialArgs = { inherit inputs; };
      
      modules = [
        # ./hardware-configuration.nix
        # ./configuration.nix
        
        # An inline module adding the package
        ({ pkgs, inputs, ... }: {
          environment.systemPackages = [
            inputs.mvis.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```
*(For Home Manager, the concept is the same: pass inputs via `extraSpecialArgs = { inherit inputs; };` and add the package to `home.packages`).*

### MPD Configuration (NixOS)
In NixOS, `services.mpd` is configured system-wide and now uses the declarative `settings` format. Ensure you set the `user` so MPD can access your user's PipeWire/PulseAudio session:

```nix
services.mpd = {
  enable = true;
  user = "your-username"; # Critical for accessing user audio socket
  
  settings = {
    music_directory = "/home/your-username/Music";
    bind_to_address = "127.0.0.1";
    port = 6600;
    
    audio_output = [
      {
        type = "pulse"; # Or "pipewire" / "alsa"
        name = "Local Audio";
      }
      {
        type   = "fifo";
        name   = "Visualizer FIFO";
        path   = "/tmp/mpd.fifo";
        format = "44100:16:2";
      }
    ];
  };
};
```

### MPD Configuration (Home Manager)
In Home Manager, MPD runs as a user service natively, so `~/Music` works and audio sockets are easily accessed.

```nix
services.mpd = {
  enable = true;
  musicDirectory = "~/Music";
  network.listenAddress = "127.0.0.1";
  network.port = 6600;
  extraConfig = ''
    # 1. Your normal audio output (e.g., PipeWire / PulseAudio)
    audio_output {
      type "pulse" # Or "pipewire" / "alsa"
      name "Local Audio"
    }

    # 2. The FIFO output required for this app's audio reactivity
    audio_output {
      type   "fifo"
      name   "Visualizer FIFO"
      path   "/tmp/mpd.fifo"
      format "44100:16:2"
    }
  '';
};
```

### Autostart with Hyprland (NixOS / Home Manager)
If you're using Home Manager, you can add `mvis-wallpaper` to your Hyprland configuration's `exec-once`:
```nix
wayland.windowManager.hyprland.settings.exec-once = [
  "${inputs.mvis.packages.${pkgs.system}.default}/bin/mvis-wallpaper"
];
```

## Standard Setup (Without Nix)

### 1. Install Build Dependencies

#### Arch Linux
You'll need the Rust toolchain, ALSA for audio handling, and standard build tools. Additionally, Bevy requires Vulkan and some X11/Wayland development libraries.

```bash
# Install the Rust toolchain and build tools
sudo pacman -S rustup pkgconf gcc alsa-lib

# Setup the default Rust toolchain
rustup default stable

# Depending on your environment, you may also need Vulkan loaders and Wayland/X11 deps
sudo pacman -S vulkan-icd-loader wayland libxkbcommon
```
*(Note: Ensure you have the appropriate Vulkan drivers installed for your GPU, e.g., `vulkan-radeon`, `vulkan-intel`, or `nvidia-utils`)*

#### Ubuntu / Debian
```bash
# Install build tools and ALSA dependencies
sudo apt update
sudo apt install -y build-essential pkg-config libasound2-dev

# Install the Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable

# Install Vulkan and Wayland/X11 dependencies for Bevy
sudo apt install -y libvulkan1 libxkbcommon-dev libwayland-dev
```

### 2. Set Up MPD (Music Player Daemon)

This application streams real-time audio analysis from MPD via a FIFO (First-In-First-Out) pipe and fetches track metadata over TCP.

1. **Install MPD and MPC (for control)**
   - **Arch**: `sudo pacman -S mpd mpc`
   - **Ubuntu**: `sudo apt install mpd mpc`

2. **Configure MPD**
   Create the local user configuration directory for MPD:
   ```bash
   mkdir -p ~/.config/mpd
   ```

   Create or edit `~/.config/mpd/mpd.conf` and ensure you have an `audio_output` block configured as a `fifo` for this app to read from. By default, the application looks for `/tmp/mpd.fifo`.

   Example `~/.config/mpd/mpd.conf`:
   ```text
   # Recommended basic directories
   music_directory    "~/Music"
   playlist_directory "~/.config/mpd/playlists"
   db_file            "~/.config/mpd/database"
   log_file           "~/.config/mpd/log"
   pid_file           "~/.config/mpd/pid"
   state_file         "~/.config/mpd/state"

   # Make sure MPD listens on localhost so the app can fetch metadata
   bind_to_address "127.0.0.1"
   port "6600"

   # 1. Your normal audio output (e.g., PipeWire / PulseAudio)
   audio_output {
       type "pulse" # Or "pipewire" / "alsa"
       name "Local Audio"
   }

   # 2. The FIFO output required for this app's audio reactivity
   audio_output {
       type   "fifo"
       name   "Visualizer FIFO"
       path   "/tmp/mpd.fifo"
       format "44100:16:2"
   }
   ```

3. **Start the MPD Service**
   Start MPD as a user service:
   ```bash
   systemctl --user enable --now mpd.service
   ```

4. **Play some music**
   Make sure you have music in your `~/Music` folder, update the database, and play it:
   ```bash
   mpc update
   mpc ls | mpc add
   mpc play
   ```

### 3. Build and Run the App

#### Using Nix & Just (Recommended)
If you have Nix installed, this project is packaged as a flake. You can easily run it using `just`:
```bash
# Run the standalone visualizer
just run

# Or build the release binaries
just build
```

#### Using Cargo
Once dependencies are installed and MPD is playing music to the FIFO, you can run the standalone app directly with Cargo:
```bash
# Run in release mode for maximum performance (recommended)
cargo run --release
```

#### Running the Wallpaper Daemon
To run the application as a live desktop wallpaper (GUI menu hidden by default):
```bash
cargo run --release --bin mvis-wallpaper
```
*(You can pass `-w /path/to/wallpaper.jpg` to manually set the background image).*

#### Controlling via CLI (`mvis-cli`)
You can control the running simulation dynamically using the `mvis-cli` utility:
```bash
# Randomize all unlocked simulation parameters
cargo run --release --bin mvis-cli -- randomize

# Lock a parameter to prevent it from being randomized
cargo run --release --bin mvis-cli -- lock "Gravity Strength"

# Unlock a parameter
cargo run --release --bin mvis-cli -- unlock "Gravity Strength"

# Set a specific parameter manually
cargo run --release --bin mvis-cli -- set "Gravity Strength" 0.5
```

### Autostart with Hyprland (Standard)
To automatically start the wallpaper daemon when Hyprland launches, add the following line to your `~/.config/hypr/hyprland.conf`:

```hyprlang
# Assuming you built it with `cargo build --release` and copied the binary to your PATH
# or using the absolute path to the compiled binary:
exec-once = /path/to/your/cloned/mvis/target/release/mvis-wallpaper
```

### Configuration

On the first run, the app will generate a `config.toml` file in the standard XDG config directory (typically `~/.config/mvis/config.toml`). You can customize the MPD connection details and default simulation parameters there.

```toml
[mpd]
host = "127.0.0.1:6600"
fifo_path = "/tmp/mpd.fifo"
```

## Credits

This project was inspired by and built upon concepts from:
- [Particle Life Simulation by OfficialCodeNoodles](https://github.com/OfficialCodeNoodles/Particle-Life-Simulation.git)
- [osu! Community Forum Topic (Archived)](https://osu.ppy.sh/community/forums/topics/1044998?n=1)
