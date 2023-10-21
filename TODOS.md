## General
- [x] If config folder or mods folder don't exit make them
- [ ] Change `String` to `PathBuf` in mine_data_struct.

## update.rs
- [ ] Fix `update_modpack` unwrap
- [ ] `update_modpack` should return a `Result<(), Error>`

## maker.rs
- [x] `make_modpack` should return a `Result<(), Error>`
- [x] Refactor `serach_mods_for_modpack` and `search_mod`
- [x] Change `path` type to `PathBuf`
- [x] Make a type alias for `&[(String, String)]`
- [x] Change `&'str` to `&Path` to everything that uses a path.
- [x] Refactor `ModpackMaker` into a struct with chunk method
- [x] Make docs for pub functions
- [ ] Make docs for non-pub functions
- [ ] Handle errors and don't panic (`unwrap` in `read_mods` and `parse_responses`)
- [x] Move `client` into the struct
- [x] Change `&'a path`  to `PathBuf`
- [x] Add `len()` method indicating how many mods are in the minecraft dir.
- [x] Make a field `threads`  in `ModpackMaker` so it doesn't need to call `N_THREADS()` fun all the time.



## pack_zipper.rs
- [x] Change paths/names types to `PathBuf` or `Path`
- [x] Fix unwraps in `compress_pack`
- [x] Change `&str` to `Path`  in `search_files`
- [ ] Change magic strings to constants (is this really necessary?)


## rinth_downloader.rs
- [x] Make docs for pub functions
- [ ] Make docs for non-pub functions

## lib.rs
- [x] Write docs for public functions
