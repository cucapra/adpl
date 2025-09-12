mod cli;

use std::borrow::Cow;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{fs, io};

use adpl::parse::parse;
use adpl::util::{Diagnostic, Reporter};

use cli::Opts;

fn read_input(file: &Option<PathBuf>) -> io::Result<(Cow<'_, str>, String)> {
    if let Some(file) = file {
        let filename = file.to_string_lossy();
        let source = fs::read_to_string(file)?;

        Ok((filename, source))
    } else {
        let filename = Cow::from("<stdin>");
        let source = io::read_to_string(io::stdin())?;

        Ok((filename, source))
    }
}

fn main() -> ExitCode {
    let opts = Opts::parse();

    let (filename, source) = match read_input(&opts.file) {
        Ok(ok) => ok,
        Err(err) => {
            Reporter::new("", "").emit(&Diagnostic::from(err));

            return ExitCode::FAILURE;
        }
    };

    let mut reporter = Reporter::new(&filename, &source);

    let _ = match parse(&source) {
        Ok(ok) => ok,
        Err(errors) => {
            for err in errors {
                reporter.emit(
                    &Diagnostic::error()
                        .with_message("syntax error")
                        .with_primary(err.span().clone(), "syntax error"),
                );
            }

            return ExitCode::FAILURE;
        }
    };

    ExitCode::SUCCESS
}
