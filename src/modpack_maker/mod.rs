mod maker;
pub use maker::{ModpackMaker, State};
/*

    TODO:
        - Estructura para analizar un profile (&Profile) y crear un modpack a partir
        de ese profile.
        - La estructura tiene que ser capaz de:
            · Saber los mods del perfil
            · Tener los mods cargados con la estructura de version_file (RinthVersionFile)
              para saber datos de la versión especifica actual.
                (https://api.modrinth.com/v2/version_file/619e250c133106bacc3e3b560839bd4b324dfda8)
            · Tener los mods cargados con la estructura de project/{slug}/version (RinthVersions)
                para saber los datos de las versiones mas nuevas del mod que sigan usando la version
                de minecraft actual.
                (https://api.modrinth.com/v2/project/Jw3Wx1KR/version)
            · Poder mostrar la version mas actualizada del mod para la versión de minecraft.
            · Usar la misma filosofia de progress() para facilitar la asincronicidad.
            · enum MakingProgress {
            ·   ReadingMods
            ·   RetrievingMods
            ·   LookingForUpdates
            ·   Finished
            · }

        Ejemplo:

            mods
              | sodium.jar
              | crate.jar
              | fabric-api.jar
              | minimap.jar

           https://api.modrinth.com/v2/project/Jw3Wx1KR/version?game_versions=["1.19"]


           {
            "property1": {
                "name": "Version 1.0.0",
                "version_number": "1.0.0",
            },

            "property2": {
                "name": "Version 1.0.0",
                "version_number": "1.0.0",
            }
           }

*/
