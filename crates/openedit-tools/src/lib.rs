pub mod sort;
pub mod case;
pub mod hash;
pub mod lines;
pub mod transform;

/// A text transformation tool operates on a string and produces a new string.
pub trait Tool: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn category(&self) -> &str;
    fn transform(&self, input: &str) -> Result<String, ToolError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Transform failed: {0}")]
    TransformFailed(String),
}
