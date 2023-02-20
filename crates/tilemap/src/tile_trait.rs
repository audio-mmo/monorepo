/// A valid tile type.
///
/// This trait is so named because other things may wish to use tiles.
///
/// Tiles must be `Eq` and `Hash`.  Practically, this can be done in two ways: no use of floating point types or
/// (carefully!) excluding floats from the Eq and Hash implementations.  In practice, avoiding floats and instead using
/// e.g. u16 on a fixed scale is by far the easiest option.  Even with internment, it is important to keep tiles small.
pub trait TileTrait: std::cmp::Eq + std::hash::Hash + Send + Sync + 'static {}
