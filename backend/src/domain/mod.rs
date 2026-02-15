// Domain layer - business logic, entities, value objects
// No dependencies on other layers

pub mod entities;
pub mod aggregates;
pub mod value_objects;
pub mod events;
pub mod apps;

pub use entities::*;
pub use value_objects::*;
// Removed unused imports: apps::*, events::*
