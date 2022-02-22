/// A version of something, usually a component.
///
/// these are primarily used for the network and other change detection, where we consider all entities "changed" at
/// process start.  Serializes as a u64.
///
/// The default impl is the minimum possible version.
///
/// As with other things we might want to save to Sqlite, the internal representation is i64.
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    derive_more::Display,
)]
#[serde(transparent)]
#[display(fmt = "v{_0}")]
pub struct Version(i64);

impl Version {
    /// The version less than all other versions.
    pub const MIN: Version = Version(i64::MIN);

    /// Get the next version after this one.
    ///
    /// Panics if this isn't possible, which is a sign of a bug: we'd have to have a u64::MAX -1 version.
    #[must_use]
    pub fn increment(&self) -> Version {
        let nv = self.0.checked_add(1).expect("We hit the max version!");
        Version(nv)
    }
}

impl Default for Version {
    fn default() -> Version {
        Version::MIN
    }
}
