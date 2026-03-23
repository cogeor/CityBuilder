//! Hot-reload infrastructure for development builds.
//!
//! When the `hot-reload` feature is enabled, the renderer can detect
//! changes to content files and re-upload textures without restart.
//!
//! Currently a stub — add `notify` crate for filesystem watching.

/// Check if any content files have been modified since last check.
/// Returns true if a reload is needed.
///
/// # Stub implementation
/// Always returns false. Enable with a filesystem watcher for actual detection.
pub fn check_content_modified() -> bool {
    false
}

/// Content paths that are monitored for changes.
pub const WATCHED_PATHS: &[&str] = &[
    "plugins/base.world/content/spriteset_buildings.rgba",
    "plugins/base.world/content/buildings.ron",
    "plugins/base.world/content/tile_visuals.ron",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_false() {
        assert!(!check_content_modified());
    }
}
