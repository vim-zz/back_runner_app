# BackRunner

BackRunner is a macOS menu bar application that helps manage SSH tunnels. It provides an easy way to open and close SSH tunnels through a simple menu interface.

## Features

- Simple menu bar interface
- Easy tunnel management
- Automatic cleanup on app termination
- Native macOS integration

## Installation

### Prerequisites

- Rust and Cargo (install via [rustup](https://rustup.rs/))
- Xcode Command Line Tools
- cargo-bundle (install with `cargo install cargo-bundle`)

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/back_runner.git
cd back_runner
```

2. Build and bundle the application:
```bash
cargo bundle --release
```

This will create a macOS application bundle in `target/release/bundle/osx/BackRunner.app`

3. Move the app to your Applications folder:
```bash
cp -r target/release/bundle/osx/BackRunner.app /Applications/
```

### Running

Simply double-click the BackRunner.app in your Applications folder or launch it from Spotlight.

The app will appear as a menu bar item with the following options:
- Open tunnel PROD
- Open tunnel DEV-01
- Quit

## License

MIT
