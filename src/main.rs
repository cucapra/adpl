mod cli;

use adpl::lex;

use cli::Opts;

fn main() {
    let opts = Opts::parse();
    let _ = opts.file;

    lex::hello();
}
