use mine_data_structs::minecraft::{AssetIndex, DownloadData, Library, ObjectData};

/// Result of a version check operation containing references to problematic
/// files.
///
/// Lifetime `'a` ties the result to the verifier's data.
pub struct VersionCheckResult<'a> {
    pub objects: Box<[&'a ObjectData]>,
    pub libs: Box<[&'a Library]>,
    pub index: Option<&'a AssetIndex>,
    pub client: Option<&'a DownloadData>,
}

impl VersionCheckResult<'_> {
    /// Returns `true` if no problems were found.
    pub fn is_valid(&self) -> bool {
        self.objects.is_empty()
            && self.libs.is_empty()
            && self.index.is_none()
            && self.client.is_none()
    }

    /// Total number of problematic items.
    pub fn total_problems(&self) -> usize {
        self.objects.len()
            + self.libs.len()
            + self
                .index
                .map(|_| 1)
                .unwrap_or_default()
            + self
                .client
                .map(|_| 1)
                .unwrap_or_default()
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn lib_count(&self) -> usize {
        self.libs.len()
    }
}
