// Domain layer - business logic, entities, value objects
// No dependencies on other layers

pub mod entities;
pub mod aggregates;
pub mod value_objects;
pub mod events;

pub use entities::*;
pub use aggregates::*;
pub use value_objects::*;
pub use events::*;
