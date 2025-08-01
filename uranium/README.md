# 🛠️ Uranium

[API Docs](https://img.shields.io/crates/v/uranium-rs.svg)


**Uranium** is a Rust library for downloading Minecraft game files and Modrinth mods or modpacks. It provides:

- A generic, asynchronous downloader
- A trait-based system so users can plug in their own downloader implementation

This crate is made for users who want to make their own minecraft launcher but
dont want to write all the process from scratch. Uranium provides an easy way
to download the minecraft files and also modpacks from ModRinth (Curse is in
progress). Uranium also has functions to interact with the default minecraft
launcher profiles, so you can read already existing profiles and add new ones
(it is interoperable with other launchers).

# How to download minecraft ?

It's that simple:

```rust
let mut downloader = MinecraftDownloader::<Downloader>::init("/home/user/.minecraft", "1.20.1").await?;
downloader.start().await;
```

# FileDownloader trait

Uranium's FileDownloader trait enables users to implement custom downloaders to
their needs. If you feel the **Downloader** Uranium provides is slow or it doesn't
satisfy your need go and write your own !

Then change the generic parameter like this and you'll be using your own downloder:
```rust
let mut downloader = MinecraftDownloader::<MyDownloader>::init("/home/user/.minecraft", "1.20.1").await?;
downloader.start().await;
```

# Rinth

**Uranium** also can download modpacks with the modrith format with `RinthDownloader`.

``` rust
RinthDownloader::<Downloader>::new("path/to/modpack", "installation/path")?;
```

# Curse

Curse is under developing right now, it has a specific downloader but with no guarantees.
Same API as Rinth:
```rust
CurseDownloader::<Downloader>::new("path/to/modpack", "installation/path").await;
```
