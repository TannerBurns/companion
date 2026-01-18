pub mod gemini;
pub mod prompts;
pub mod pipeline;

pub use gemini::{GeminiClient, ServiceAccountCredentials};
pub use pipeline::ProcessingPipeline;
pub use prompts::{SummaryResult, DigestSummary};
