//! Pure analysis passes over the universal symbol model (`GB-203` step 2).
//!
//! "Pure" is the boundary: everything here takes symbols and returns a report, with no store, no
//! persistence and no engine behind it. The passes that walk the redb tables - contracts, cycles,
//! hygiene, impact and the dirty ingest - stay in `miter-engine` beside the store they read, and
//! `audit` and `deps` follow `miter-inventory`, because they call into it.
//!
//! The engine re-exports these modules under `analysis`, so no caller changes a path.

pub mod clones;
pub mod deps;
pub mod cluster;
pub mod references;
pub mod skeleton;
pub mod slice;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_clones;
#[cfg(test)]
mod tests_skeleton;
