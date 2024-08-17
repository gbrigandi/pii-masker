pub mod similarity;

#[derive(Debug)]
pub enum MaskerError {
    RuleParseError(String),
    SimilarityError(String),
}

pub type MResult<T> = std::result::Result<T, MaskerError>;


