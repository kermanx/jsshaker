use oxc::{
  allocator::{self, Allocator},
  semantic::SymbolId,
};

use super::{get::GetPropertyContext, set::PendingSetter};
use crate::{
  analyzer::Analyzer,
  dep::{Dep, DepCollector},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  utils::Found,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectPropertyKey<'a> {
  String(&'a str),
  Symbol(SymbolId),
}

#[derive(Debug, Clone, Copy)]
pub enum ObjectPropertyValue<'a> {
  /// (value, readonly)
  Field(Entity<'a>, bool),
  /// (getter, setter)
  Property(Option<Entity<'a>>, Option<Entity<'a>>),
}

#[derive(Debug)]
pub struct ObjectProperty<'a> {
  pub consumed: bool,
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
      consumed: false,
      definite: true,
      enumerable: true,
      possible_values: allocator::Vec::new_in(allocator),
      non_existent: DepCollector::new(allocator::Vec::new_in(allocator)),
      key: None,
      mangling: None,
    }
  }

  pub(super) fn may_be_field(&self) -> bool {
    if !self.definite {
      return true;
    }
    for possible_value in &self.possible_values {
      if matches!(possible_value, ObjectPropertyValue::Field(_, _)) {
        return true;
      }
    }
    false
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
    if let Some(dep) = self.non_existent.try_collect(analyzer.factory) {
      context.extra_deps.push(dep);
    }
  }

  fn get_unmangable(&mut self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => context.values.push(if self.consumed {
          analyzer.factory.computed_unknown(*value)
        } else {
          *value
        }),
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
    let constraint = MangleConstraint::Eq(prev_atom, key_atom);
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => {
          let value = analyzer.factory.mangable(*value, (prev_key, context.key), constraint);
          context.values.push(if self.consumed {
            analyzer.factory.computed_unknown(value)
          } else {
            value
          });
        }
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
    setters: &mut Vec<PendingSetter<'a>>,
  ) {
    let mut writable = false;
    for possible_value in &self.possible_values {
      match *possible_value {
        ObjectPropertyValue::Field(_, readonly) if !readonly => writable = true,
        ObjectPropertyValue::Property(_, Some(setter)) => setters.push(PendingSetter {
          indeterminate: self.possible_values.len() > 1,
          dep: self.non_existent.collect(analyzer.factory),
          setter,
        }),
        _ => {}
      }
    }

    if writable {
      if !indeterminate {
        // Remove all writable fields
        self
          .possible_values
          .retain(|possible_value| !matches!(possible_value, ObjectPropertyValue::Field(_, false)));
        // This property must exist now
        self.definite = true;
        self.non_existent.force_clear();
      }

      self.possible_values.push(ObjectPropertyValue::Field(value, false));
    }
  }

  pub fn lookup_setters(
    &mut self,
    analyzer: &Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    let mut found_setter = false;
    let mut found_others = false;
    for possible_value in &self.possible_values {
      if let ObjectPropertyValue::Property(_, Some(setter)) = *possible_value {
        setters.push(PendingSetter {
          indeterminate: self.possible_values.len() > 1,
          dep: self.non_existent.collect(analyzer.factory),
          setter,
        });
        found_setter = true;
      } else {
        found_others = false;
      }
    }
    if found_others { Found::Unknown } else { Found::known(found_setter) }
  }

  pub fn delete(&mut self, indeterminate: bool, dep: Dep<'a>) {
    self.definite = false;
    if !indeterminate {
      self.possible_values.clear();
      self.non_existent.force_clear();
    }
    self.non_existent.push(dep);
  }

  pub fn consume(
    &self,
    analyzer: &mut Analyzer<'a>,
    suspended: &mut allocator::Vec<'a, Entity<'a>>,
  ) {
    for &possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => suspended.push(value),
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

    self.non_existent.consume_all(analyzer);

    if let Some(key) = self.key {
      suspended.push(key);
    }
  }
}
