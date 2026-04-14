pub mod server;
pub mod protocol;
pub use server::AgentServer;
pub use protocol::{AgentRequest, AgentResponse as IpcResponse, AgentCapability};
