use std::num::NonZeroU64;

/// A version of something, usually a component.
///
/// these are primarily used for the network and other change detection, where we consider all entities "changed" at
/// process start.  Serializes as a u64.
///
/// The default impl is the minimum possible version.
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
pub struct Version(NonZeroU64);

impl Version {
    /// The version less than all other versions.
    pub const MIN: Version = Version(unsafe { NonZeroU64::new_unchecked(1) });
    /// The maximum version ever allowed.
    pub const MAX: Version = Version(unsafe { NonZeroU64::new_unchecked(u64::MAX) });

    /// Get the nth next version.
    ///
    /// Panics if this isn't possible, which is a sign of a bug: we'd have to have a u64::MAX -1 version.
    #[must_use]
    pub fn increment_multi(&self, times: u64) -> Version {
        let nv = unsafe {
            NonZeroU64::new_unchecked(
                self.0
                    .get()
                    .checked_add(times)
                    .expect("We hit the max version!"),
            )
        };
        Version(nv)
    }

    #[must_use]
    pub fn increment(&self) -> Version {
        self.increment_multi(1)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::MIN
    }
}
