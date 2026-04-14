pub mod loader;
pub mod executor;
pub use loader::{Skill, SkillRegistry};
pub use executor::execute_skill;
