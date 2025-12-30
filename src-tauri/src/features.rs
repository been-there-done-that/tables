// Centralized feature flags for the Tauri backend.
// Set to `true` to enable a feature at compile time.
// Keep these minimal and explicit to avoid env-driven behavior.
pub struct FeatureFlags {
    pub metrics_enabled: bool,
}

pub const FEATURE_FLAGS: FeatureFlags = FeatureFlags {
    metrics_enabled: false,
};
