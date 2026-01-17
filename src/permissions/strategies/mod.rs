//! Permission mode strategy implementations
//!
//! Each strategy encapsulates the permission logic for a specific mode,
//! providing a single source of truth for how tools should be checked.

mod strategy_trait;
mod bypass_permissions_mode;
mod accept_edits_mode;
mod default_mode;
mod dont_ask_mode;
mod plan_mode;

pub use strategy_trait::PermissionModeStrategy;
pub use bypass_permissions_mode::BypassPermissionsModeStrategy;
pub use accept_edits_mode::AcceptEditsModeStrategy;
pub use default_mode::DefaultModeStrategy;
pub use dont_ask_mode::DontAskModeStrategy;
pub use plan_mode::PlanModeStrategy;
