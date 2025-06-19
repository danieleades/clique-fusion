#![doc = include_str!("../README.md")]

mod observation;
pub use observation::Observation;
pub use observation::{
    CHI2_2D_CONFIDENCE_90, CHI2_2D_CONFIDENCE_95, CHI2_2D_CONFIDENCE_99, CovarianceMatrix,
    InvalidCovarianceMatrix,
};

mod spatial_index;
pub use spatial_index::Unique;

mod clique_index;
mod cliques;
pub use clique_index::CliqueIndex;
