mod cli;
mod errors;
mod expand;

use std::process::ExitCode;

use adpl::ast_lowering::lower_ast;
use adpl::typing::check_hir;
use adpl::util::Reporter;

use cli::Opts;
use expand::resolve_imports;

fn main() -> ExitCode {
    let opts = Opts::parse();
    let mut reporter = Reporter::new();

    let Some(ast) = resolve_imports(opts.file.as_deref(), &mut reporter) else {
        return ExitCode::FAILURE;
    };

    let Some(hir) = lower_ast(&ast, &mut reporter) else {
        return ExitCode::FAILURE;
    };

    let Some(_) = check_hir(&hir, &mut reporter) else {
        return ExitCode::FAILURE;
    };

    ExitCode::SUCCESS
}
