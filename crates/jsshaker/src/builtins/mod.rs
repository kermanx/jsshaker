mod globals;
mod import_meta;
mod known_modules;
mod prototypes;
mod react;
mod utils;

use known_modules::KnownModule;
pub use prototypes::BuiltinPrototype;
use prototypes::BuiltinPrototypes;
use react::AnalyzerDataForReact;
use rustc_hash::FxHashMap;

use crate::{TreeShakeConfig, analyzer::Factory, entity::Entity};

pub struct Builtins<'a> {
  pub config: &'a TreeShakeConfig,
  pub factory: &'a Factory<'a>,

  pub prototypes: &'a BuiltinPrototypes<'a>,
  pub globals: FxHashMap<&'static str, Entity<'a>>,
  pub known_modules: FxHashMap<&'static str, KnownModule<'a>>,

  pub react_data: AnalyzerDataForReact<'a>,
}

impl<'a> Builtins<'a> {
  pub fn new(config: &'a TreeShakeConfig, factory: &'a Factory<'a>) -> Self {
    let prototypes = Self::create_builtin_prototypes(factory);
    let mut builtins = Self {
      config,
      factory,

      prototypes,
      globals: Default::default(),       // Initialize later
      known_modules: Default::default(), // Initialize later

      react_data: Default::default(),
    };
    builtins.init_globals();
    builtins.init_known_modules();
    builtins
  }
}
