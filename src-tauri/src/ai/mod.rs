pub mod gemini;
pub mod pipeline;
pub mod prompts;

pub use gemini::{GeminiClient, ServiceAccountCredentials};
pub use pipeline::ProcessingPipeline;
pub use prompts::{DigestSummary, SummaryResult};
