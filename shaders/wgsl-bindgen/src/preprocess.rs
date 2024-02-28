use std::{
    io,
    path::{
        Path,
        PathBuf,
    },
};

use thiserror::Error;

const INSTRUCTION_PREFIX: &str = "//!";
const INCLUDE_INSTRUCTION: &str = "include";

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct ShaderBuilder {
    src: PathBuf,
}

pub struct ProcessedShader {
    code: String,
    includes: Vec<PathBuf>,
}

impl ProcessedShader {
    pub fn wgsl(self) -> String {
        self.code
    }

    pub fn includes(&self) -> impl Iterator<Item = &Path> {
        self.includes.iter().map(|p| p.as_path())
    }
}

impl ShaderBuilder {
    pub fn new(src: &Path) -> Self {
        Self {
            src: src.to_owned(),
        }
    }

    pub fn build(self) -> Result<ProcessedShader, Error> {
        let (entire_module, includes) = process(self.src)?;

        Ok(ProcessedShader {
            code: entire_module,
            includes,
        })
    }
}

fn process(src: impl AsRef<Path>) -> Result<(String, Vec<PathBuf>), io::Error> {
    return inner(src.as_ref());

    fn inner(src: &Path) -> Result<(String, Vec<PathBuf>), io::Error> {
        let parent = src.parent();
        let module_source = std::fs::read_to_string(src)?;

        let mut module_string = String::new();
        let mut includes = Vec::new();

        'next_line: for line in module_source.lines() {
            if let Some(rest) = line.strip_prefix(INSTRUCTION_PREFIX) {
                if rest.starts_with(INCLUDE_INSTRUCTION) {
                    for include in rest.split_whitespace().skip(1) {
                        let mut include_path = PathBuf::new();
                        if let Some(parent) = parent {
                            include_path.push(parent);
                        }
                        include_path.push(include);

                        let (included_module_string, mut other_includes) = process(&include_path)?;

                        includes.push(include_path);

                        module_string.push_str(&included_module_string);
                        includes.append(&mut other_includes);
                    }

                    continue 'next_line;
                }
            }

            module_string.push_str(line);
            module_string.push('\n');
        }

        Ok((module_string, includes))
    }
}
