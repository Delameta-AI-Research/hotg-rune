use std::{
    collections::HashSet,
    ffi::OsStr,
    fmt::{self, Debug, Display, Formatter},
    path::{Path, PathBuf},
};
use anyhow::{Error, Context};
use cargo_toml::{Manifest, Package, Publish};
use walkdir::WalkDir;

const AUTHORS: &[&str] = &["The Rune Developers <admin@hotg.ai>"];
const LICENSE: &str = "MIT OR Apache-2.0";
const HOMEPAGE: &str = "https://hotg.dev/";
const REPOSITORY: &str = "https://github.com/hotg-ai/rune";

#[derive(Debug, structopt::StructOpt)]
pub struct CheckManifests {}

impl CheckManifests {
    pub fn run(self, project_root: &Path) -> Result<(), Error> {
        let entries = WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| e.file_name() != OsStr::new("target"));

        let mut crates = Vec::new();

        for entry in entries {
            let entry = entry?;
            if entry.file_name() != OsStr::new("Cargo.toml") {
                continue;
            }

            let cargo_toml = entry.path();

            match CrateInfo::from_path(project_root, cargo_toml) {
                Ok(Some(info)) => {
                    crates.push(info);
                },
                Ok(None) => {
                    log::debug!("Skipping \"{}\"", cargo_toml.display())
                },
                Err(e) => {
                    return Err(e.context(format!(
                        "Unable to get crate info for \"{}\"",
                        cargo_toml.display()
                    )))
                },
            }
        }

        let mut diagnostics: Vec<_> = crates
            .iter()
            .filter_map(|c| check_manifest(c))
            .map(Error::from)
            .collect();

        let versions: HashSet<_> =
            crates.iter().map(|c| c.package.version.clone()).collect();

        if versions.len() != 1 {
            let e = anyhow::anyhow!("All published crates should have the same version, but found {:?}", versions);
            diagnostics.push(e);
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(Error::from(BulkErrors(diagnostics)))
        }
    }
}

#[derive(Debug)]
struct BulkErrors(Vec<Error>);

impl std::error::Error for BulkErrors {}

impl Display for BulkErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0.len() != 1 {
            writeln!(f, "Several issues were found.")?;
            writeln!(f)?;
        }

        for error in &self.0 {
            writeln!(f, "{}", error)?;
        }

        Ok(())
    }
}

fn check_manifest(info: &CrateInfo) -> Option<Diagnostics> {
    let CrateInfo {
        short_path,
        package:
            Package {
                authors,
                license,
                categories,
                keywords,
                description,
                homepage,
                repository,
                readme,
                ..
            },
        ..
    } = info;

    log::debug!("Checking \"{}\"", short_path.display());

    let mut expect = Diagnostics::new(&short_path);

    expect.array_field("Authors", &authors).to_equal(AUTHORS);
    expect.array_field("Categories", &categories).is_not_empty();
    expect.array_field("Keywords", &keywords).is_not_empty();
    expect.field("Description", description.as_deref()).is_set();
    expect
        .field("README", readme.as_deref())
        .is_set_to("README.md");
    expect
        .field("License", license.as_deref())
        .is_set_to(LICENSE);
    expect
        .field("Homepage", homepage.as_deref())
        .is_set_to(HOMEPAGE);
    expect
        .field("Repository", repository.as_deref())
        .is_set_to(REPOSITORY);

    if expect.has_errors() {
        Some(expect)
    } else {
        None
    }
}

#[derive(Debug)]
struct Diagnostics {
    cargo_toml: PathBuf,
    messages: Vec<Diagnostic>,
}

impl Diagnostics {
    fn new(cargo_toml: &Path) -> Self {
        Diagnostics {
            cargo_toml: cargo_toml.to_path_buf(),
            messages: Vec::new(),
        }
    }

    fn array_field<'diag, 'value, T>(
        &'diag mut self,
        field: &'static str,
        actual: &'value [T],
    ) -> ExpectArray<'diag, 'value, T> {
        ExpectArray {
            diags: self,
            field,
            actual,
        }
    }

    fn field<'diag, 'value>(
        &'diag mut self,
        field: &'static str,
        actual: Option<&'value str>,
    ) -> Expect<'diag, 'value> {
        Expect {
            diags: self,
            field,
            actual,
        }
    }

    fn push(&mut self, field: &'static str, message: impl Into<String>) {
        self.messages.push(Diagnostic {
            field,
            message: message.into(),
        });
    }

    fn has_errors(&self) -> bool { !self.messages.is_empty() }
}

impl std::error::Error for Diagnostics {}

impl Display for Diagnostics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.messages.as_slice() {
            [] => {},
            [message] => {
                writeln!(
                    f,
                    "There was 1 issue with \"{}\"",
                    self.cargo_toml.display()
                )?;
                writeln!(f, "  {}", message)?;
            },
            [messages @ ..] => {
                writeln!(
                    f,
                    "There were {} issues with \"{}\"",
                    messages.len(),
                    self.cargo_toml.display()
                )?;

                for message in messages {
                    writeln!(f, "  {}", message)?;
                }
            },
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ExpectArray<'diag, 'value, T> {
    diags: &'diag mut Diagnostics,
    field: &'static str,
    actual: &'value [T],
}

impl<'diag, 'value, T: Debug> ExpectArray<'diag, 'value, T> {
    fn to_equal<V>(self, value: &[V])
    where
        V: PartialEq<T> + Debug,
    {
        if value != self.actual {
            self.diags.push(
                self.field,
                format!("should be {:?} but found {:?}", value, self.actual),
            );
        }
    }

    fn is_not_empty(self) {
        if self.actual.is_empty() {
            self.diags
                .push(self.field, "shouldn't be empty".to_string());
        }
    }
}

#[derive(Debug)]
struct Diagnostic {
    field: &'static str,
    message: String,
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.field, self.message)
    }
}

struct Expect<'diag, 'value> {
    diags: &'diag mut Diagnostics,
    field: &'static str,
    actual: Option<&'value str>,
}

impl<'diag, 'value> Expect<'diag, 'value> {
    fn is_set(self) {
        if self.actual.is_none() {
            self.diags.push(self.field, "should be set");
        }
    }

    fn is_set_to(self, should_be: &str) {
        match self.actual {
            Some(value) if value == should_be => {},
            Some(other_value) => {
                self.diags.push(
                    self.field,
                    format!(
                        "should be set to \"{}\", found \"{}\"",
                        should_be, other_value
                    ),
                );
            },
            None => {
                self.diags.push(
                    self.field,
                    format!("should be set to \"{}\"", should_be),
                );
            },
        }
    }
}

struct CrateInfo {
    short_path: PathBuf,
    package: Package,
}

impl CrateInfo {
    fn from_path(
        project_root: &Path,
        cargo_toml: &Path,
    ) -> Result<Option<Self>, Error> {
        let stripped = cargo_toml.strip_prefix(project_root)?;

        let manifest = Manifest::from_path(cargo_toml).with_context(|| {
            format!("Unable to parse \"{}\"", cargo_toml.display())
        })?;

        let package = match manifest.package {
            Some(p) => p,
            _ => return Ok(None),
        };

        if !matches!(package.publish, Publish::Flag(true)) {
            return Ok(None);
        }

        Ok(Some(CrateInfo {
            short_path: stripped.to_path_buf(),
            package,
        }))
    }
}