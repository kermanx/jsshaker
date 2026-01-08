use std::collections::HashMap;

pub trait Vfs {
  fn resolve_module(&self, importer: &str, specifier: &str) -> Option<String>;
  fn read_file(&self, path: &str) -> String;
}

pub struct StdFs;

impl Vfs for StdFs {
  fn resolve_module(&self, importer: &str, specifier: &str) -> Option<String> {
    if !specifier.starts_with(".") {
      return None;
    }

    let mut path = std::path::PathBuf::from(importer);
    path.pop();
    path.push(specifier);
    path = normalize_path::normalize(&path);
    let result = path
      .exists()
      .then(|| path.to_string_lossy().into_owned())
      .or_else(|| {
        path.set_extension("js");
        path.exists().then(|| path.to_string_lossy().into_owned())
      })
      .or_else(|| {
        path.set_extension("mjs");
        path.exists().then(|| path.to_string_lossy().into_owned())
      })
      .or_else(|| {
        path.set_extension("cjs");
        path.exists().then(|| path.to_string_lossy().into_owned())
      });

    if result.is_none() {
      eprintln!("[Warning] Cannot resolve module: {} from {}", specifier, importer);
    }

    result
  }

  fn read_file(&self, path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
  }
}

pub struct SingleFileFs(pub String);

impl SingleFileFs {
  pub const ENTRY_PATH: &'static str = "/entry.js";
}

impl Vfs for SingleFileFs {
  fn resolve_module(&self, _importer: &str, _specifier: &str) -> Option<String> {
    None
  }

  fn read_file(&self, path: &str) -> String {
    if path == Self::ENTRY_PATH {
      self.0.clone()
    } else {
      unreachable!("Unexpected path: {}", path);
    }
  }
}

pub struct MultiModuleFs(pub HashMap<String, String>);

impl MultiModuleFs {
  fn exists(&self, path: &std::path::Path) -> bool {
    self.0.contains_key(path.to_str().unwrap())
  }
}

impl Vfs for MultiModuleFs {
  fn resolve_module(&self, importer: &str, specifier: &str) -> Option<String> {
    if !specifier.starts_with(".") {
      return None;
    }

    let mut path = std::path::PathBuf::from(importer);
    path.pop();
    path.push(specifier);
    path = normalize_path::normalize(&path);
    Some(
      self
        .exists(&path)
        .then(|| path.to_string_lossy().into_owned())
        .or_else(|| {
          path.set_extension("js");
          self.exists(&path).then(|| path.to_string_lossy().into_owned())
        })
        .or_else(|| {
          path.set_extension("mjs");
          self.exists(&path).then(|| path.to_string_lossy().into_owned())
        })
        .or_else(|| {
          path.set_extension("cjs");
          self.exists(&path).then(|| path.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| {
          panic!(
            "Cannot resolve module: {} from {}. (concated path: {})",
            specifier,
            importer,
            path.to_string_lossy()
          );
        }),
    )
  }

  fn read_file(&self, path: &str) -> String {
    self.0.get(path).cloned().unwrap()
  }
}

// Credit: https://github.com/rust-lang/rfcs/issues/2208#issuecomment-342679694
pub mod normalize_path {
  use std::path::Component;
  use std::path::Path;
  use std::path::PathBuf;

  pub fn normalize_str(p: &str) -> String {
    normalize(&PathBuf::from(p)).to_string_lossy().into_owned()
  }

  pub fn normalize(p: &Path) -> PathBuf {
    let mut stack: Vec<Component> = vec![];

    // We assume .components() removes redundant consecutive path separators.
    // Note that .components() also does some normalization of '.' on its own anyways.
    // This '.' normalization happens to be compatible with the approach below.
    for component in p.components() {
      match component {
        // Drop CurDir components, do not even push onto the stack.
        Component::CurDir => {}

        // For ParentDir components, we need to use the contents of the stack.
        Component::ParentDir => {
          // Look at the top element of stack, if any.
          let top = stack.last().cloned();

          match top {
            // A component is on the stack, need more pattern matching.
            Some(c) => {
              match c {
                // Push the ParentDir on the stack.
                Component::Prefix(_) => {
                  stack.push(component);
                }

                // The parent of a RootDir is itself, so drop the ParentDir (no-op).
                Component::RootDir => {}

                // A CurDir should never be found on the stack, since they are dropped when seen.
                Component::CurDir => {
                  unreachable!();
                }

                // If a ParentDir is found, it must be due to it piling up at the start of a path.
                // Push the new ParentDir onto the stack.
                Component::ParentDir => {
                  stack.push(component);
                }

                // If a Normal is found, pop it off.
                Component::Normal(_) => {
                  let _ = stack.pop();
                }
              }
            }

            // Stack is empty, so path is empty, just push.
            None => {
              stack.push(component);
            }
          }
        }

        // All others, simply push onto the stack.
        _ => {
          stack.push(component);
        }
      }
    }

    // If an empty PathBuf would be return, instead return CurDir ('.').
    if stack.is_empty() {
      return PathBuf::from(&Component::CurDir);
    }

    let mut norm_path = PathBuf::new();

    for item in &stack {
      norm_path.push(item);
    }

    norm_path
  }
}
