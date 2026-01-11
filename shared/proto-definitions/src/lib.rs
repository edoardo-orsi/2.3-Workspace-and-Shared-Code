// Explicitly marks the crate as "used"
#[allow(unused_imports)]
use prost as _;

#[allow(unused_imports)]
use tonic_prost as _;

pub mod generated_protos;

pub use generated_protos::*;