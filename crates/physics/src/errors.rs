#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AabbError {
    #[error("Attempted to create an AABB which would have an invalid width or height")]
    AabbInvalidDims,

    #[error("This operation cannot be completed because the dimensions of this AABB cause one of the points to be beyond a U16's range")]
    AabbU16Overflow,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("AAbb error: {}", _0)]
    Aabb(#[from] AabbError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
