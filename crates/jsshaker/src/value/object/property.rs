use oxc::allocator::{self, Allocator};

use super::{get::GetPropertyContext, set::PendingSetter};
use crate::{
  analyzer::Analyzer,
  dep::{Dep, DepCollector, LazyDep},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint, ManglingDep},
  utils::Found,
};

#[derive(Debug, Clone, Copy)]
pub enum ObjectPropertyValue<'a> {
  Consumed(Entity<'a>, LazyDep<'a, Entity<'a>>),
  /// (value, readonly)
  Field(Entity<'a>, bool),
  /// (getter, setter)
  Property(Option<Entity<'a>>, Option<Entity<'a>>),
}

impl<'a> ObjectPropertyValue<'a> {
  pub fn new_included(analyzer: &Analyzer<'a>, deps: allocator::Vec<'a, Entity<'a>>) -> Self {
    let deps = analyzer.factory.lazy_dep(deps);
    Self::Consumed(analyzer.factory.computed_unknown(deps), deps)
  }
}

#[derive(Debug)]
pub struct ObjectProperty<'a> {
  /// Does this property definitely exist
  pub definite: bool,
  /// Is this property enumerable
  pub enumerable: bool,
  /// Possible values of this property
  pub possible_values: allocator::Vec<'a, ObjectPropertyValue<'a>>,
  /// Why this property is non-existent
  pub non_existent: DepCollector<'a>,
  /// The key entity. None if it is just Literal(key)
  pub key: Option<Entity<'a>>,
  /// key_atom if this property's key is mangable
  pub mangling: Option<MangleAtom>,
}

impl<'a> ObjectProperty<'a> {
  pub fn new_in(allocator: &'a Allocator) -> Self {
    Self {
      definite: true,
      enumerable: true,
      possible_values: allocator::Vec::new_in(allocator),
      non_existent: DepCollector::new(allocator::Vec::new_in(allocator)),
      key: None,
      mangling: None,
    }
  }

  pub(super) fn may_be_unincluded_field(&self) -> bool {
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Consumed(_, _) => return false,
        ObjectPropertyValue::Field(_, false) => return true,
        _ => {}
      }
    }
    !self.definite
  }

  pub fn make_mangling_dep(&mut self, key: Entity<'a>, key_atom: MangleAtom) -> ManglingDep<'a> {
    let prev_key = self.key.unwrap();
    let prev_atom = self.mangling.unwrap();
    ManglingDep { deps: (prev_key, key), constraint: MangleConstraint::Eq(prev_atom, key_atom) }
  }

  pub(super) fn get(
    &mut self,
    analyzer: &Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
    key_atom: Option<MangleAtom>,
  ) {
    if let Some(key_atom) = key_atom {
      self.get_mangable(analyzer, context, key_atom);
    } else {
      self.get_unmangable(analyzer, context);
    }
    if let Some(dep) = self.non_existent.collect(analyzer.factory) {
      context.extra_deps.push(dep);
    }
  }

  fn get_mangable(
    &mut self,
    analyzer: &Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
    key_atom: MangleAtom,
  ) {
    let prev_key = self.key.unwrap();
    let prev_atom = self.mangling.unwrap();
    let constraint = MangleConstraint::Eq(prev_atom, key_atom);
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Consumed(value, _) | ObjectPropertyValue::Field(value, _) => context
          .values
          .push(analyzer.factory.mangable(*value, (prev_key, context.key), constraint)),
        ObjectPropertyValue::Property(Some(getter), _) => context
          .getters
          .push(analyzer.factory.mangable(*getter, (prev_key, context.key), constraint)),
        ObjectPropertyValue::Property(None, _) => context.values.push(analyzer.factory.mangable(
          analyzer.factory.undefined,
          (prev_key, context.key),
          constraint,
        )),
      }
    }
  }

  fn get_unmangable(&mut self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    let computed = |v: Entity<'a>| {
      if context.mangable && self.mangling.is_none() {
        analyzer.factory.computed(v, (context.key, self.key))
      } else {
        analyzer.factory.optional_computed(v, self.key)
      }
    };

    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Consumed(value, _) | ObjectPropertyValue::Field(value, _) => {
          context.values.push(computed(*value))
        }
        ObjectPropertyValue::Property(Some(getter), _) => context.getters.push(computed(*getter)),
        ObjectPropertyValue::Property(None, _) => {
          context.values.push(computed(analyzer.factory.undefined))
        }
      }
    }
  }

  pub(super) fn get_value(
    &mut self,
    analyzer: &Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
  ) {
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Consumed(value, _) | ObjectPropertyValue::Field(value, _) => {
          context.values.push(*value)
        }
        ObjectPropertyValue::Property(Some(getter), _) => context.getters.push(*getter),
        ObjectPropertyValue::Property(None, _) => context.values.push(analyzer.factory.undefined),
      }
    }
    if let Some(dep) = self.non_existent.collect(analyzer.factory) {
      context.extra_deps.push(dep);
    }
  }

  pub fn set(
    &mut self,
    analyzer: &mut Analyzer<'a>,
    is_exhaustive: bool,
    is_private_identifier: bool,
    non_det: bool,
    key: Entity<'a>,
    key_atom: Option<MangleAtom>,
    mut value: Entity<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
    deferred_deps: &mut Vec<Entity<'a>>,
  ) -> bool {
    let mangling_dep = key_atom.map(|key_atom| self.make_mangling_dep(key, key_atom));

    if non_det || is_private_identifier {
      value = analyzer.factory.optional_computed(value, mangling_dep);
    } else if let Some(key_atom) = key_atom {
      let prev_atom = self.mangling.unwrap();
      value = analyzer.factory.computed(value, MangleConstraint::Eq(prev_atom, key_atom));
    }

    let mut writable = false;
    let mut was_included = None;
    let mut field_values = vec![value];

    for possible_value in &self.possible_values {
      match *possible_value {
        ObjectPropertyValue::Consumed(_, deps) => {
          writable = true;
          was_included = Some(deps);
        }
        ObjectPropertyValue::Field(value, readonly) => {
          writable |= !readonly;
          field_values.push(value);
        }
        ObjectPropertyValue::Property(_, Some(setter)) => setters.push(PendingSetter {
          non_det: self.possible_values.len() > 1 || !self.definite,
          dep: self.non_existent.collect(analyzer.factory),
          setter: analyzer.factory.optional_computed(setter, mangling_dep),
        }),
        ObjectPropertyValue::Property(_, None) => {}
      }
    }

    if writable {
      if !non_det || is_exhaustive {
        // Remove all writable fields
        self
          .possible_values
          .retain(|possible_value| !matches!(possible_value, ObjectPropertyValue::Field(_, false)));
      }

      if !non_det {
        // This property must exist now
        self.definite = true;
        self.non_existent.force_clear();

        if !is_private_identifier {
          self.key = Some(key);
          self.mangling = key_atom;
        }
      }

      if is_exhaustive {
        if let Some(was_included) = was_included {
          for field_value in field_values {
            was_included.push_defer(field_value, deferred_deps);
          }
        } else {
          self.possible_values.push(ObjectPropertyValue::new_included(
            analyzer,
            allocator::Vec::from_iter_in(field_values, analyzer.allocator),
          ));
        }
      } else {
        self.possible_values.push(ObjectPropertyValue::Field(value, false));
      }
    }

    writable && was_included.is_none()
  }

  pub fn lookup_setters(
    &mut self,
    analyzer: &Analyzer<'a>,
    mangling_dep: Option<ManglingDep<'a>>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    let mut found_setter = false;
    let mut found_others = false;
    for possible_value in &self.possible_values {
      if let ObjectPropertyValue::Property(_, Some(setter)) = *possible_value {
        setters.push(PendingSetter {
          non_det: self.possible_values.len() > 1,
          dep: self.non_existent.collect(analyzer.factory),
          setter: analyzer.factory.optional_computed(setter, mangling_dep),
        });
        found_setter = true;
      } else {
        found_others = false;
      }
    }
    if found_others { Found::Unknown } else { Found::known(found_setter) }
  }

  pub fn delete(&mut self, non_det: bool, dep: Dep<'a>) {
    self.definite = false;
    if !non_det {
      self.possible_values.clear();
      self.non_existent.force_clear();
    }
    self.non_existent.push(dep);
  }

  pub fn include(
    &self,
    analyzer: &mut Analyzer<'a>,
    suspended: &mut allocator::Vec<'a, Entity<'a>>,
  ) {
    for &possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Consumed(value, _) | ObjectPropertyValue::Field(value, _) => {
          suspended.push(value)
        }
        ObjectPropertyValue::Property(getter, setter) => {
          if let Some(getter) = getter {
            suspended.push(getter);
          }
          if let Some(setter) = setter {
            suspended.push(setter);
          }
        }
      }
    }

    self.non_existent.include_all(analyzer);

    if let Some(key) = self.key {
      suspended.push(key);
    }
  }
}
