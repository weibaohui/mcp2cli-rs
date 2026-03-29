//! Domain layer - Core business logic and entities
//!
//! This layer contains:
//! - Domain entities and value objects
//! - Domain errors
//! - Repository interfaces (traits)
//! - Domain services

pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

pub use entities::*;
pub use errors::*;
pub use repositories::*;
pub use services::*;
pub use value_objects::*;
