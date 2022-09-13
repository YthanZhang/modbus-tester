use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrKind {
    MathOperationParseError,
    RequestParseError,
    InvalidPortOption,
    MathOperationResultInOutOfRangeValue,

    FailedToOpenTargetPort,
    PortWriteFailed,

    PortOpThreadNotPresent,
    PortOpDroppedChannelTxWithoutResponse,

    PortTypeUnequal,

    AttemptToStartMultipleContinuousQuarry,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    kind: ErrKind,
    message: String,
}

impl Error {
    /// Create a error with no message
    pub fn new(kind: ErrKind) -> Self {
        Self { kind, message: "".to_string() }
    }

    /// Create a error with custom message
    pub fn with_message(kind: ErrKind, message: String) -> Self {
        Self { kind, message }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {{{:?}, {}}}", self.kind, self.message)
    }
}

impl std::error::Error for Error {}
