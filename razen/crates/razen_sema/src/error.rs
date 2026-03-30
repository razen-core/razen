//! Semantic errors for Name Resolution and Type Checking.

use razen_ast::span::Span;
use std::fmt;

/// An error encountered during semantic analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// An identifier was used but not defined in any visible scope.
    UndefinedIdentifier {
        name: String,
        span: Span,
    },
    /// A name was defined multiple times in a way that violates scoping rules.
    AlreadyDefined {
        name: String,
        span: Span,
        previous_span: Span,
    },
    /// An imported item could not be found.
    UnresolvedImport {
        path: String,
        span: Span,
    },
    /// General semantic error.
    Custom {
        message: String,
        span: Span,
    },
}

impl SemanticError {
    pub fn span(&self) -> Span {
        match self {
            Self::UndefinedIdentifier { span, .. } => *span,
            Self::AlreadyDefined { span, .. } => *span,
            Self::UnresolvedImport { span, .. } => *span,
            Self::Custom { span, .. } => *span,
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndefinedIdentifier { name, .. } => {
                write!(f, "Cannot find value, function, or type `{}` in this scope", name)
            }
            Self::AlreadyDefined { name, .. } => {
                write!(f, "The name `{}` is defined multiple times", name)
            }
            Self::UnresolvedImport { path, .. } => {
                write!(f, "Failed to resolve import `{}`", path)
            }
            Self::Custom { message, .. } => write!(f, "{}", message),
        }
    }
}
