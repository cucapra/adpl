mod cli;

use cli::Opts;

fn main() {
    let opts = Opts::parse();
    let _ = opts.file;
}
