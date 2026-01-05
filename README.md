# Vampire Survivors Game Practice - Rust & Bevy

🎮 **A Vampire Survivors-inspired game built with Rust and Bevy Engine**

> **Note:** This project is purely for entertainment and learning purposes. It is currently a work in progress.

## 📖 About

This is a practice project implementing a Vampire Survivors-style survival game using the Bevy game engine. The game features wave-based enemy spawning, character progression with upgrades, and various weapon systems.

## 🚀 Features

### Current Implementation

- **Player Movement**: WASD controls with animated character sprites
- **Enemy System**: 
  - Automated enemy spawning with increasing difficulty
  - Enemies follow and chase the player
  - Progressive power scaling over time
- **Weapon Systems**:
  - Laser weapons with customizable colors
  - Rocket/projectile weapons
  - Player-attached weapons (shields, etc.)
  - Auto-firing mechanics
- **Progression System**:
  - XP collection from defeated enemies
  - Level-up mechanics
  - Weapon upgrade selection system
- **Game States**:
  - Loading screen
  - Active gameplay
  - Upgrade selection menu
  - Game over screen with restart capability
- **Score Tracking**: Real-time score display
- **Camera System**: Smooth camera following the player
- **Infinite Ground**: Dynamic ground chunk generation

## 🎯 Controls

- **W/A/S/D**: Move character
- **R**: Restart game (when game over)
- **Mouse**: Select weapon upgrades during level-up

## 🛠️ Technical Stack

- **Language**: Rust (Edition 2024)
- **Game Engine**: Bevy 0.17.3
- **Dependencies**:
  - `bevy_ecs` - Entity Component System
  - `bevy_light` - Lighting system
  - `rand` - Random number generation

## 📦 Building and Running

### Prerequisites

- Rust toolchain (latest stable version)
- Cargo package manager

### Build and Run

```bash
# Clone the repository
git clone https://github.com/TunahanYavuz/VampireSurvivals_Game_Practice_Rust_Bevy.git
cd VampireSurvivals_Game_Practice_Rust_Bevy

# Run in development mode
cargo run

# Run in release mode (optimized)
cargo run --release
```

### Build Profiles

The project includes optimized build profiles:
- **Development**: Basic optimization (opt-level 1) for faster compilation
- **Dependencies**: Highly optimized (opt-level 3) even in dev mode
- **Release**: Full optimization (opt-level 3)

## 🎨 Project Structure

```
src/
├── main.rs                    # Main game loop and system setup
└── plugins/
    ├── player.rs              # Player movement and behavior
    ├── enemy.rs               # Enemy spawning and AI
    ├── weapons.rs             # Weapon systems and firing
    ├── weapon_stats.rs        # Weapon configuration
    ├── weapon_upgrade.rs      # Upgrade selection system
    ├── timers.rs              # Game timing and spawn rates
    ├── aabb.rs                # Collision detection
    ├── game_state.rs          # Game state management
    ├── score.rs               # Score tracking and UI
    ├── ground.rs              # Ground generation
    └── texture_handling.rs    # Asset management
```

## 🎮 Gameplay

Survive against waves of enemies by moving around and collecting XP. As you level up, choose from random weapon upgrades to enhance your arsenal. Each upgrade improves your weapons or adds new ones to your character. The game becomes progressively harder as enemies spawn more frequently and become more powerful.

## 🚧 Work in Progress

This project is actively being developed. Future improvements may include:
- Additional weapon types
- More enemy varieties
- Power-up items
- Sound effects and music
- Visual effects enhancements
- Boss encounters
- Difficulty levels

## 📝 License

This project is for educational and entertainment purposes only.

## 🤝 Contributing

As this is a personal practice project, contributions are not currently being accepted. However, feel free to fork and experiment with your own ideas!

## 📬 Contact

Project by: [TunahanYavuz](https://github.com/TunahanYavuz)

---

*Built with ❤️ using Rust and Bevy*
