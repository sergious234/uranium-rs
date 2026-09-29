//! Single-file install phases for the `.mrpack` pipeline.

// region:    --- Modules

mod files;
mod loader;
mod overrides;
mod vanilla;
mod verify;

pub(crate) use files::FilesStep;
pub(crate) use loader::LoaderStep;
pub(crate) use overrides::OverridesStep;
pub(crate) use vanilla::VanillaStep;
pub(crate) use verify::VerifyStep;

// endregion: --- Modules
