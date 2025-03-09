use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
};

use super::get::GetPropertyContext;

#[derive(Debug, Clone, Copy)]
pub enum ObjectPropertyValue<'a> {
  /// (value, readonly)
  Field(Entity<'a>, bool),
  /// (getter, setter)
  Property(Option<Entity<'a>>, Option<Entity<'a>>),
}

#[derive(Debug)]
pub struct ObjectProperty<'a> {
  /// Does this property definitely exist
  pub definite: bool,
  /// Is this property enumerable
  pub enumerable: bool,
  /// Possible values of this property
  pub possible_values: Vec<ObjectPropertyValue<'a>>,
  /// Why this property is non-existent
  pub non_existent: ConsumableCollector<'a>,
  /// The key entity. None if it is just LiteralEntity(key)
  pub key: Option<Entity<'a>>,
  /// key_atom if this property's key is mangable
  pub mangling: Option<MangleAtom>,
}

impl<'a> Default for ObjectProperty<'a> {
  fn default() -> Self {
    Self {
      definite: true,
      enumerable: true,
      possible_values: vec![],
      non_existent: ConsumableCollector::default(),
      key: None,
      mangling: None,
    }
  }
}

impl<'a> ObjectProperty<'a> {
  pub(super) fn get(
    &mut self,
    analyzer: &Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
    key_atom: Option<MangleAtom>,
  ) -> bool {
    if let Some(key_atom) = key_atom {
      self.get_mangable(analyzer, context, key_atom);
    } else {
      self.get_unmangable(analyzer, context);
    }
    if let Some(dep) = self.non_existent.try_collect(analyzer.factory) {
      context.extra_deps.push(dep);
    }
    self.definite
  }

  fn get_unmangable(&mut self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => context.values.push(*value),
        ObjectPropertyValue::Property(Some(getter), _) => context.getters.push(*getter),
        ObjectPropertyValue::Property(None, _) => context.values.push(analyzer.factory.undefined),
      }
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
    let constraint = &*analyzer.factory.alloc(MangleConstraint::Eq(prev_atom, key_atom));
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => context.values.push(analyzer.factory.mangable(
          *value,
          (prev_key, context.key),
          constraint,
        )),
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

  pub fn set(
    &mut self,
    analyzer: &Analyzer<'a>,
    indeterminate: bool,
    value: Entity<'a>,
    setters: &mut Vec<(bool, Consumable<'a>, Entity<'a>)>,
  ) {
    let mut writable = false;
    let call_setter_indeterminately = indeterminate || self.possible_values.len() > 1;
    for possible_value in &self.possible_values {
      match *possible_value {
        ObjectPropertyValue::Field(_, false) => writable = true,
        ObjectPropertyValue::Property(_, Some(setter)) => setters.push((
          call_setter_indeterminately,
          self.non_existent.collect(analyzer.factory),
          setter,
        )),
        _ => {}
      }
    }

    if !indeterminate {
      // Remove all writable fields
      self.possible_values = self
        .possible_values
        .iter()
        .filter(|possible_value| !matches!(possible_value, ObjectPropertyValue::Field(_, false)))
        .cloned()
        .collect();
      // This property must exist now
      self.non_existent.force_clear();
    }

    if writable {
      self.possible_values.push(ObjectPropertyValue::Field(value, false));
    }
  }

  pub fn delete(&mut self, indeterminate: bool, dep: Consumable<'a>) {
    self.definite = false;
    if !indeterminate {
      self.possible_values.clear();
      self.non_existent.force_clear();
    }
    self.non_existent.push(dep);
  }

  pub fn consume(self, analyzer: &mut Analyzer<'a>) {
    for possible_value in self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => analyzer.consume(value),
        ObjectPropertyValue::Property(getter, setter) => {
          analyzer.consume(getter);
          analyzer.consume(setter);
        }
      }
    }

    self.non_existent.consume_all(analyzer);

    if let Some(key) = self.key {
      analyzer.consume(key);
    }
  }
}
