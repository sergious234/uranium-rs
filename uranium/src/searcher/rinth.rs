use std::fmt::{Display, Formatter};

/// A type for representing that no search type is set.
type NoSearchType = ();

/// A list specifying the different kinds of requests based on the API
/// routes.
#[derive(Debug, Clone)]
pub enum SearchType {
    /// /search
    Search,
    /// /project
    Project { id: String },
    /// /project/{id|slug}/version
    ProjectVersion { id: String },
    /// /projects
    MultiProject { ids: Vec<&'static str> },
    /// /version_file/{hash}
    VersionFile { hash: String },
    /// /project/{id|slug}/dependencies
    Dependencies { id: String },
    /// /tag/category
    Categories,
    /// /tag/loader
    Loaders,
}

/// A builder for building the URL with the indicated parameters
/// This struct works with TypeState Programming so the `build_url()` method
/// can't be called unless search_type is set.
///
/// The `facets` field works as a conjunction (AND) of disjunctions (OR):
///
/// I.e: (pseudocode)
/// ```pseudo
/// facets = {
///     FacetsDisjunctions(version="1.20", version="1.21"),
///     FacetsDisjunctions(category=fabric)
/// }
/// ```
///
/// That means: (version = 1.20 **OR** 1.21) **AND** (category = fabric)
pub struct SearchBuilder<T> {
    search_type: T,
    facets: Option<Vec<FacetsDisjunction>>,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    game_versions: Vec<String>,
}

impl SearchBuilder<NoSearchType> {
    pub fn new() -> SearchBuilder<NoSearchType> {
        SearchBuilder {
            search_type: (),
            facets: None,
            limit: None,
            offset: None,
            query: None,
            game_versions: vec![],
        }
    }
}

impl<T> SearchBuilder<T> {
    pub fn facets(mut self, facets: Vec<FacetsDisjunction>) -> Self {
        self.facets = Some(facets);
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the game versions filter for the project version search.
    ///
    /// This method allows you to specify the game versions for which you want
    /// to filter project versions. It accepts a vector of strings, where
    /// each string represents a specific game version (e.g., `"1.16.5"`,
    /// `"1.17.1"`).
    ///
    /// # Parameters
    ///
    /// - `versions`: A vector of strings representing the game versions to
    ///   filter by.
    ///
    /// # Returns
    ///
    /// Returns an updated instance of the `SearchBuilder` with the specified
    /// game versions filter applied.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium::searcher::rinth::{SearchType, SearchBuilder};
    /// let builder = SearchBuilder::new()
    ///     .search_type(SearchType::ProjectVersion {id: "example_id".to_owned()})
    ///     .game_versions(vec!["1.16.5".to_string(), "1.17.1".to_string()])
    ///     .build_url();
    /// ```
    ///
    /// # Restrictions
    ///
    /// This method is only available when the search type is `ProjectVersion`.
    /// Attempting to call this method for other search types will do nothing.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Errors
    ///
    /// This method does not return errors.
    ///
    /// # Notes
    ///
    /// - The `game_versions` parameter allows you to filter results to only
    ///   include project versions compatible with the specified game versions.
    /// - Ensure that you are using this method within the `ProjectVersion`
    ///   search context, as it is specifically designed for filtering versions
    ///   based on game compatibility.
    pub fn game_versions(mut self, versions: Vec<String>) -> Self {
        self.game_versions = versions;
        self
    }

    /// Adds a single game version to the game versions filter for the project
    /// version search.
    ///
    /// This method allows you to append a game version to the existing filter
    /// criteria in the `SearchBuilder`. The version is added to the vector
    /// of game versions that will be used to filter the results of the
    /// search.
    ///
    /// # Parameters
    ///
    /// - `version`: A string slice representing a single game version (e.g.,
    ///   `"1.17.1"`) to be added to the game versions filter.
    ///
    /// # Returns
    ///
    /// Returns an updated instance of the `SearchBuilder` with the specified
    /// game version added to the filter criteria.
    ///
    /// # Example
    ///
    /// ```rust no_run
    /// use uranium::searcher::rinth::{SearchType, SearchBuilder};
    /// let builder = SearchBuilder::new()
    ///     .search_type(SearchType::ProjectVersion {id: "example_id".to_string()})
    ///     .add_game_version("1.16.5")
    ///     .add_game_version("1.17.1")
    ///     .build_url();
    /// ```
    ///
    /// # Restrictions
    ///
    /// This method only has an effect when the search type is `ProjectVersion`.
    /// If used with a different search type, it will have no impact on the
    /// search builder and will silently ignore the call.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Errors
    ///
    /// This method does not return errors.
    ///
    /// # Notes
    /// - Ensure that this method is called within the `ProjectVersion` search
    ///   context, as it is specifically designed for filtering project versions
    ///   based on game compatibility.
    /// - If used outside the `ProjectVersion` context, the method will not
    ///   modify the builder and will effectively do nothing.
    pub fn add_game_version(mut self, version: &str) -> Self {
        self.game_versions
            .push(version.to_owned());
        self
    }

    pub fn search_type(self, search_type: SearchType) -> SearchBuilder<SearchType> {
        SearchBuilder {
            search_type,
            query: self.query,
            facets: self.facets,
            offset: self.offset,
            limit: self.limit,
            game_versions: self.game_versions,
        }
    }
}

impl SearchBuilder<SearchType> {
    /// Generates the URL based on the `SearchBuilder` object.
    ///
    /// This method constructs a URL for the Modrinth API using various
    /// parameters from the `SearchBuilder` struct. It supports multiple
    /// search types and query parameters, and it constructs the URL
    /// accordingly.
    ///
    /// # Returns
    ///
    /// A `String` representing the constructed URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use uranium::searcher::rinth::{SearchBuilder, SearchType};
    /// let search_builder: String = SearchBuilder::new()
    ///     .limit(10)
    ///     .offset(5)
    ///     .search_type(SearchType::Search)
    ///     .build_url();
    /// assert_eq!("https://api.modrinth.com/v2/search?limit=10&offset=5", &search_builder);
    /// ```
    pub fn build_url(self) -> String {
        use std::mem::discriminant;
        let mut url: String = "https://api.modrinth.com/v2/".to_string();

        let component = match &self.search_type {
            SearchType::Project { id } => &format!("project/{id}?"),
            SearchType::MultiProject { ids } => {
                let ids = ids
                    .iter()
                    .map(|id| format!("\"{id}\""))
                    .collect::<Vec<String>>()
                    .join(",");

                &format!("projects?ids=[{ids}]")
            }
            SearchType::Search => "search?",
            SearchType::VersionFile { hash } => &format!("version_file/{hash}"),
            SearchType::Dependencies { .. } => todo!(),

            // If SearchType is Categories or Loaders there is no need to apply
            // queries/facets...
            SearchType::Categories => {
                url.push_str("tag/category");
                return url;
            }
            SearchType::Loaders => {
                url.push_str("tag/loader");
                return url;
            }
            SearchType::ProjectVersion { id } => &format!("project/{id}/version"),
        };
        url.push_str(component);

        if !self.game_versions.is_empty()
            && discriminant(&self.search_type)
                == discriminant(&SearchType::ProjectVersion { id: "".to_string() })
        {
            url.push('?');
            url.push_str("game_versions=[");
            for version in self.game_versions {
                url.push_str(&format!("\"{version}\","))
            }
            // Remove trailing comma
            url.pop();
            url.push(']');

            // Since ProjectVersion doesn't accept facets, offset or limit
            // return is a right thing to do.
            return url;
        }

        if let Some(query) = self.query {
            url.push_str(format!("query={query}&").as_str())
        }

        if let Some(limit) = self.limit {
            url.push_str(format!("limit={limit}&").as_str())
        }

        if let Some(offset) = self.offset {
            url.push_str(format!("offset={offset}&").as_str())
        }

        if let Some(facets) = self.facets {
            url.push_str("facets=[");
            for conjunction in facets {
                url.push_str("[");
                for face in conjunction.facets {
                    url.push_str(format!("{face},").as_str())
                }
                // Remove the trailing comma
                url.pop();
                url.push_str("],");
            }
            // Remove the trailing comma
            url.pop();
            url.push(']');
            url.push('&');
        }

        if url.ends_with('&') {
            url.pop();
        }

        url
    }
}

/// This struct represent a disjunction (OR) of facets.
#[derive(Debug, Clone)]
pub struct FacetsDisjunction {
    facets: Vec<Facets>,
}

impl FacetsDisjunction {
    pub fn new() -> Self {
        Self { facets: vec![] }
    }

    pub fn push(&mut self, facet: Facets) {
        self.facets.push(facet)
    }
}

/// A list specifying the different kinds of facets/filters that can be applied
/// to queries.
#[derive(Debug, Clone)]
pub enum Facets {
    ProjectType(String),
    Categories(String),
    Version(String),
    ClientSide(Requirement),
    ServerSide(Requirement),
    OpenSource,
}

/// A list specifying the different kinds of requirements types.
#[derive(Debug, Copy, Clone)]
pub enum Requirement {
    Optional,
    Required,
    Unsupported,
}

impl Display for Requirement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Optional => "optional",
            Self::Required => "required",
            Self::Unsupported => "unsupported",
        };
        f.write_str(s)
    }
}

impl Display for Facets {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Facets::ProjectType(t) => format!("\"project_type:{t}\""),
            Facets::Categories(c) => format!("\"categories:{c}\""),
            Facets::Version(v) => format!("\"versions:{v}\""),
            Facets::ClientSide(r) => format!("\"client_side:{r}\""),
            Facets::ServerSide(r) => format!("\"server_side:{r}\""),
            Facets::OpenSource => todo!(),
        };
        f.write_str(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use mine_data_structs::rinth::RinthCategories;
    use reqwest::{ClientBuilder, Method};

    use super::*;
    use crate::searcher::rinth::SearchType::Categories;

    #[test]
    pub fn search_builder() {
        let url = SearchBuilder::new()
            .offset(10)
            .limit(5)
            .search_type(SearchType::Search)
            .build_url();

        assert_eq!("https://api.modrinth.com/v2/search?limit=5&offset=10", url)
    }

    #[test]
    pub fn search_builder_facets() {
        let mut versions_facets = FacetsDisjunction::new();
        versions_facets.push(Facets::Version("1.21".to_string()));
        versions_facets.push(Facets::Version("1.20".to_string()));

        let url = SearchBuilder::new()
            .offset(10)
            .limit(5)
            .facets(vec![versions_facets])
            .search_type(SearchType::Search)
            .build_url();
        assert_eq!(
            "https://api.modrinth.com/v2/search?limit=5&offset=10&facets=[\
                [\"versions:1.21\",\"versions:1.20\"]\
            ]",
            url
        );
    }

    #[test]
    pub fn search_builder_facets_disjunction() {
        let mut versions_facets = FacetsDisjunction::new();
        versions_facets.push(Facets::Version("1.21".to_string()));
        versions_facets.push(Facets::Version("1.20".to_string()));

        let mut type_facets = FacetsDisjunction::new();
        type_facets.push(Facets::ProjectType("modpack".to_string()));

        let url = SearchBuilder::new()
            .offset(10)
            .limit(5)
            .facets(vec![versions_facets, type_facets])
            .search_type(SearchType::Search)
            .build_url();

        assert_eq!(
            "https://api.modrinth.com/v2/search?limit=5&offset=10&facets=[\
                [\"versions:1.21\",\"versions:1.20\"],\
                [\"project_type:modpack\"]\
            ]",
            url
        );
    }

    #[test]
    pub fn search_builder_facets_disjunction2() {
        let mut versions_facets = FacetsDisjunction::new();
        versions_facets.push(Facets::Version("1.19".to_string()));
        versions_facets.push(Facets::Version("1.22".to_string()));

        let mut type_facets = FacetsDisjunction::new();
        type_facets.push(Facets::ProjectType("modpack".to_string()));

        let mut categories_facets = FacetsDisjunction::new();
        categories_facets.push(Facets::Categories("technology".to_string()));
        categories_facets.push(Facets::Categories("adventure".to_string()));

        let url = SearchBuilder::new()
            .offset(10)
            .limit(5)
            .facets(vec![versions_facets, type_facets, categories_facets])
            .search_type(SearchType::Search)
            .build_url();

        assert_eq!(
            "https://api.modrinth.com/v2/search?limit=5&offset=10&facets=[\
                [\"versions:1.19\",\"versions:1.22\"],\
                [\"project_type:modpack\"],\
                [\"categories:technology\",\"categories:adventure\"]\
            ]",
            url
        );
    }

    #[test]
    pub fn search_builder_projects() {
        let url = SearchBuilder::new()
            .search_type(SearchType::MultiProject {
                ids: vec!["AAA", "BBB"],
            })
            .build_url();

        assert_eq!(
            "https://api.modrinth.com/v2/projects?ids=[\"AAA\",\"BBB\"]",
            url
        )
    }

    #[test]
    pub fn search_builder_project_versions() {
        let url = SearchBuilder::new()
            .search_type(SearchType::ProjectVersion {
                id: "Jw3Wx1KR".to_string(),
            })
            .game_versions(vec!["1.18".to_string(), "1.18.2".to_string()])
            .build_url();

        assert_eq!("https://api.modrinth.com/v2/project/Jw3Wx1KR/version?game_versions=[\"1.18\",\"1.18.2\"]",
        url);
    }
    #[tokio::test]
    pub async fn search_categories() {
        let url = SearchBuilder::new()
            .search_type(Categories)
            .build_url();

        let categories: RinthCategories = ClientBuilder::new()
            .build()
            .unwrap()
            .get(&url)
            .header(
                "User-Agent",
                "sergious234/uranium-rs (tests)/ (sergious234@gmail.com)",
            )
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        println!("{:?}", categories);
        assert!(!categories.is_empty())
    }

    #[test]
    pub fn request_builder() {
        let c = reqwest::Client::new()
            .request(Method::GET, "https://api.modrinth.com/v2/search")
            .query(&[("query", "pokemon")])
            .query(&[("offset", 10)])
            .query(&[("limit", 100)])
            .build()
            .unwrap();

        assert_eq!(
            c.url().as_str(),
            "https://api.modrinth.com/v2/search?query=pokemon&offset=10&limit=100"
        )
    }
}
