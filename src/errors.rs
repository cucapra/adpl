use std::io;

use adpl::parse;
use adpl::util::Diagnostic;

pub struct IoError(pub io::Error);

impl From<IoError> for Diagnostic {
    fn from(value: IoError) -> Self {
        Diagnostic::error().with_message(value.0.to_string())
    }
}

pub struct ParseError(pub parse::Error);

impl From<ParseError> for Diagnostic {
    fn from(value: ParseError) -> Self {
        let (file, range) = value.0.span().clone();

        Diagnostic::error()
            .with_message("syntax error")
            .with_primary((file.into(), range), "syntax error")
    }
}
