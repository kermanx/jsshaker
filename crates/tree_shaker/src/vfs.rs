pub trait Vfs {
  fn resolve_module(&self, importer: &str, specifier: &str) -> Option<String>;
  fn read_file(&self, path: &str) -> String;
  fn normalize_path(&self, path: String) -> String;
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
    path
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
      })
  }

  fn read_file(&self, path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
  }

  fn normalize_path(&self, path: String) -> String {
    path.to_lowercase().replace("\\", "/").replace("./", "")
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

  fn normalize_path(&self, path: String) -> String {
    path
  }
}
