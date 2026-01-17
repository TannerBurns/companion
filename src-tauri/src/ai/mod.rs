pub mod gemini;
pub mod prompts;
pub mod pipeline;

pub use gemini::GeminiClient;
pub use pipeline::ProcessingPipeline;
pub use prompts::{SummaryResult, DigestSummary};
