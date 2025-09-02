use std::path::PathBuf;

/// The ADPL compiler.
#[derive(argh::FromArgs)]
pub struct Opts {
    /// input file
    #[argh(positional)]
    pub file: Option<PathBuf>,
}

impl Opts {
    /// Parses options from `env::args`.
    pub fn parse() -> Opts {
        argh::from_env()
    }
}
