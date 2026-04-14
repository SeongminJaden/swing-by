//! Discord remote control bot
//!
//! Environment variables:
//!   DISCORD_TOKEN       bot token (required)
//!   DISCORD_PREFIX      command prefix (default: "!")
//!   DISCORD_CHANNEL_ID  allowed channel ID (empty = all channels allowed)
//!
//! Supported commands:
//!   !ask <question>     ask the AI
//!   !code <request>     code generation request (developer agent)
//!   !plan <task>        task planning (planner agent)
//!   !debug <problem>    bug analysis (debugger agent)
//!   !pipeline <task>    full pipeline: plan→dev→debug
//!   !status             agent status
//!   !clear              clear current channel session
//!   !history [n]        conversation history

pub mod bot;
pub mod session;

pub use bot::run_discord_bot;
