pub mod ai;
pub mod crawler;
pub mod filters;
mod http;
pub mod markdown;
pub mod project;
pub mod robots;
pub mod state;
#[cfg(feature = "desktop")]
pub mod ui;
pub mod visualization;

/// Native packages keep their existing identity. Flatpak supplies its own ID.
pub const DESKTOP_APP_ID: &str = "com.tadpole.seo";

pub fn application_id(flatpak_id: Option<&str>) -> &str {
    flatpak_id
        .filter(|id| !id.is_empty())
        .unwrap_or(DESKTOP_APP_ID)
}

#[cfg(test)]
mod application_identity_tests {
    #[test]
    fn identity_matches_the_distribution() {
        assert_eq!(super::application_id(None), super::DESKTOP_APP_ID);
        assert_eq!(super::application_id(Some("")), super::DESKTOP_APP_ID);
        assert_eq!(
            super::application_id(Some("io.github.example.Sample")),
            "io.github.example.Sample"
        );
    }
}
