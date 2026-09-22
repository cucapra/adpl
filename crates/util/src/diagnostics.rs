use std::io::{self, IsTerminal};
use std::ops::Range;
use std::sync::LazyLock;

use codespan_reporting::diagnostic::{Diagnostic as InnerDiagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self, Config};

pub struct Diagnostic(InnerDiagnostic<usize>);

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
        S: Into<(usize, Range<usize>)>,
        L: Into<String>,
    {
        let (file, range) = span.into();

        self.0
            .labels
            .push(Label::primary(file, range).with_message(label));

        self
    }

    pub fn with_secondary<S, L>(mut self, span: S, label: L) -> Diagnostic
    where
        S: Into<(usize, Range<usize>)>,
        L: Into<String>,
    {
        let (file, range) = span.into();

        self.0
            .labels
            .push(Label::secondary(file, range).with_message(label));

        self
    }

    pub fn with_note<N: Into<String>>(mut self, note: N) -> Diagnostic {
        self.0.notes.push(note.into());

        self
    }
}

pub struct Reporter {
    files: SimpleFiles<String, String>,
    writer: StandardStream,
}

impl Reporter {
    pub fn new() -> Reporter {
        let choice = if io::stderr().is_terminal() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };

        Reporter {
            files: SimpleFiles::new(),
            writer: StandardStream::stderr(choice),
        }
    }

    pub fn add_file(&mut self, name: String, source: String) -> (usize, &str) {
        let handle = self.files.add(name, source);

        (handle, self.files.get(handle).unwrap().source())
    }

    pub fn emit_diagnostic(&mut self, diagnostic: &Diagnostic) {
        term::emit(
            &mut self.writer,
            Reporter::config(),
            &self.files,
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

impl Default for Reporter {
    fn default() -> Self {
        Reporter::new()
    }
}
