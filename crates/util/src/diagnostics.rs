use std::io::{self, IsTerminal};
use std::ops::Range;
use std::sync::LazyLock;

use codespan_reporting::diagnostic::{Diagnostic as InnerDiagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self, Config};

pub struct Diagnostic(InnerDiagnostic<()>);

impl Diagnostic {
    #[inline]
    pub fn bug() -> Diagnostic {
        Self(InnerDiagnostic::bug())
    }

    #[inline]
    pub fn error() -> Diagnostic {
        Self(InnerDiagnostic::error())
    }

    #[inline]
    pub fn warning() -> Diagnostic {
        Self(InnerDiagnostic::warning())
    }

    pub fn with_message<M: Into<String>>(mut self, message: M) -> Diagnostic {
        self.0.message = message.into();
        self
    }

    pub fn with_primary<S, L>(mut self, span: S, label: L) -> Diagnostic
    where
        S: Into<Range<usize>>,
        L: Into<String>,
    {
        self.0
            .labels
            .push(Label::primary((), span).with_message(label));

        self
    }

    pub fn with_secondary<S, L>(mut self, span: S, label: L) -> Diagnostic
    where
        S: Into<Range<usize>>,
        L: Into<String>,
    {
        self.0
            .labels
            .push(Label::secondary((), span).with_message(label));

        self
    }

    pub fn with_note<N: Into<String>>(mut self, note: N) -> Diagnostic {
        self.0.notes.push(note.into());
        self
    }
}

pub struct Reporter<'src> {
    file: SimpleFile<&'src str, &'src str>,
    writer: StandardStream,
}

impl<'src> Reporter<'src> {
    pub fn early() -> Reporter<'static> {
        Reporter::new("", "")
    }

    pub fn new(filename: &'src str, source: &'src str) -> Reporter<'src> {
        let choice = if io::stderr().is_terminal() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };

        Reporter {
            file: SimpleFile::new(filename, source),
            writer: StandardStream::stderr(choice),
        }
    }

    pub fn emit_diagnostic(&mut self, diagnostic: &Diagnostic) {
        term::emit(
            &mut self.writer,
            Reporter::config(),
            &self.file,
            &diagnostic.0,
        )
        .unwrap();
    }

    pub fn emit<D: Into<Diagnostic>>(&mut self, diagnostic: D) {
        self.emit_diagnostic(&diagnostic.into());
    }

    fn config() -> &'static Config {
        static CONFIG: LazyLock<Config> = LazyLock::new(Config::default);

        &CONFIG
    }
}
