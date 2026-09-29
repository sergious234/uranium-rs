use std::fs::File;
use std::path::Path;

use log::{error, info};
use mine_data_structs::minecraft::{Profile, ProfilesJson};

use crate::common::constants::PROFILES_FILE;
use crate::error::{Result, UraniumError};

// region:    --- Profile helpers (moved from installer.rs:713)

pub fn add_instance(
    minecraft_path: &Path,
    minecraft_id: &str,
    instance_name: &str,
    icon: Option<&str>,
) -> Result<()> {
    let profiles_path = minecraft_path
        .to_path_buf()
        .join(PROFILES_FILE);

    if !profiles_path.exists() {
        error!("{profiles_path:?} doesn't exist!");
        return Err(UraniumError::FileNotFound(
            profiles_path
                .display()
                .to_string(),
        ));
    }

    let mut profiles: ProfilesJson = match serde_json::from_reader(File::open(&profiles_path)?) {
        Ok(v) => v,
        Err(e) => Err(UraniumError::OtherWithReason(e.to_string()))?,
    };

    let icon = icon.unwrap_or("Grass");
    let new_profile = Profile::new(
        icon,
        minecraft_id,
        instance_name,
        "custom",
        Some(minecraft_path),
    );
    profiles.insert(instance_name, new_profile);

    info!("Writing new profile");

    let Ok(content) = serde_json::to_string_pretty(&profiles) else {
        return Err(UraniumError::WrongFileFormat);
    };

    if let Err(err) = std::fs::write(profiles_path, content) {
        error!("Error writing the new profile");
        return Err(err.into());
    }

    info!("Profile added!");
    Ok(())
}

// endregion: --- Profile helpers
