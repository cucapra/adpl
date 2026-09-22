use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use adpl::ast;
use adpl::parse::parse;
use adpl::util::Reporter;

use crate::errors;

fn read_stdin() -> io::Result<(String, String)> {
    let name = String::from("<stdin>");
    let source = io::read_to_string(io::stdin())?;

    Ok((name, source))
}

fn parse_file(
    name: String,
    source: String,
    reporter: &mut Reporter,
) -> Option<ast::File> {
    let (file, source) = reporter.add_file(name, source);

    parse(file.try_into().unwrap(), source)
        .map_err(|errors| {
            for err in errors {
                reporter.emit(errors::ParseError(err));
            }
        })
        .ok()
}

enum Action {
    Explore(PathBuf),
    Finish(Vec<ast::Item>),
}

#[derive(Default)]
struct Resolver {
    pending: Vec<Action>,
    imported: HashSet<PathBuf>,
}

impl Resolver {
    fn read_file(
        &mut self,
        path: &Path,
    ) -> io::Result<Option<(String, String)>> {
        if self.imported.insert(path.canonicalize()?) {
            let name = path.to_string_lossy().into_owned();
            let source = fs::read_to_string(path)?;

            Ok(Some((name, source)))
        } else {
            Ok(None)
        }
    }

    fn add_with<F: Fn(String) -> PathBuf>(&mut self, file: ast::File, f: F) {
        self.pending.push(Action::Finish(file.items));

        self.pending.extend(
            file.imports
                .into_iter()
                .rev()
                .map(|import| Action::Explore(f(import))),
        );
    }

    fn add(&mut self, file: ast::File) {
        self.add_with(file, PathBuf::from);
    }

    fn add_relative(&mut self, parent: &Path, file: ast::File) {
        self.add_with(file, |import| parent.join(import));
    }
}

pub fn resolve_imports(
    root: Option<&Path>,
    reporter: &mut Reporter,
) -> Option<Vec<ast::Item>> {
    let mut resolver = Resolver::default();
    let mut items = Vec::new();

    if let Some(path) = root {
        resolver.pending.push(Action::Explore(path.to_path_buf()));
    } else {
        let (name, source) = read_stdin()
            .map_err(|err| reporter.emit(errors::IoError(err)))
            .ok()?;

        resolver.add(parse_file(name, source, reporter)?);
    }

    while let Some(action) = resolver.pending.pop() {
        match action {
            Action::Explore(path) => {
                if let Some((name, source)) = resolver
                    .read_file(&path)
                    .map_err(|err| reporter.emit(errors::IoError(err)))
                    .ok()?
                {
                    let file = parse_file(name, source, reporter)?;
                    let parent = path.parent().unwrap();

                    resolver.add_relative(parent, file);
                }
            }
            Action::Finish(mut post) => {
                items.append(&mut post);
            }
        }
    }

    Some(items)
}
