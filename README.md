# 🛠️ Uranium
uranium-rs: 
[![Crates.io](https://img.shields.io/crates/v/uranium-rs.svg)](https://crates.io/crates/uranium-rs)

mine_data_structs: 
[![Crates.io](https://img.shields.io/crates/v/mine_data_structs.svg)](https://crates.io/crates/mine_data_structs)

**Uranium** is a Rust library for downloading Minecraft game files and Modrinth
mods or modpacks. It provides a complete toolkit for building Minecraft
launchers without having to implement the download logic from scratch.

## Features

- **Generic, asynchronous downloader** - Fast and efficient file downloading
- **Trait-based system** - Plug in your own downloader implementation
- **Minecraft file management** - Download game files for any version
- **ModRinth support** - Download modpacks from ModRinth
- **CurseForge support** - Download modpacks from CurseForge (experimental)
- **Profile integration** - Read and write Minecraft launcher profiles

This crate is designed for developers who want to create their own Minecraft
launcher but don't want to write all the downloading and file management logic
from scratch. Uranium provides an easy way to download Minecraft files and
modpacks while remaining interoperable with the default Minecraft launcher and
other launchers.

## Quick Start

### Downloading Minecraft

```rust
let mut downloader = MinecraftDownloader::<Downloader>::init("/home/user/.minecraft", "1.20.1").await?;
downloader.start().await;
```

### Downloading ModRinth Modpacks

```rust
let downloader = RinthDownloader::<Downloader>::new("path/to/modpack", "installation/path")?;
```

### Downloading CurseForge Modpacks (Experimental)

```rust
let downloader = CurseDownloader::<Downloader>::new("path/to/modpack", "installation/path").await;
```

## Custom Downloaders

Uranium's `FileDownloader` trait enables you to implement custom downloaders
tailored to your needs. If the built-in `Downloader` doesn't meet your
requirements, you can write your own:

```rust
let mut downloader = MinecraftDownloader::<MyDownloader>::init("/home/user/.minecraft", "1.20.1").await?;
downloader.start().await;
```

## Project Structure

This project consists of multiple submodules:

- **`uranium-rs`** - The main library containing all downloading and file management functionality
- **`mine_data_structs`** - Supporting crate containing data structures used by uranium-rs

## Status

- ✅ **Minecraft downloading** - Stable and fully functional
- ✅ **ModRinth support** - Stable and fully functional  
- ⚠️ **CurseForge support** - Under development, experimental
- ✅ **Profile management** - Stable and fully functional

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
uranium-rs = "0.1.0"  # Replace with actual version
```

## Documentation

[API Documentation](https://docs.rs/uranium-rs) 

## Contributing

This project was created for personal use, but contributions are welcome if you find it useful.
