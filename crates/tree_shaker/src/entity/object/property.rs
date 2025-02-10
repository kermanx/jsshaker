use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector, ConsumableVec},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
};

#[derive(Debug, Clone, Copy)]
pub enum ObjectPropertyValue<'a> {
  /// (value, readonly)
  Field(Entity<'a>, bool),
  /// (getter, setter)
  Property(Option<Entity<'a>>, Option<Entity<'a>>),
}

#[derive(Debug)]
pub struct ObjectProperty<'a> {
  pub definite: bool,                                // 是否一定存在
  pub possible_values: Vec<ObjectPropertyValue<'a>>, // 可能的值，可能有多个
  pub non_existent: ConsumableCollector<'a>,         // 如果不存在，为什么？
  pub mangling: Option<(Entity<'a>, MangleAtom)>,
}

impl Default for ObjectProperty<'_> {
  fn default() -> Self {
    Self {
      definite: true,
      possible_values: vec![],
      non_existent: ConsumableCollector::default(),
      mangling: None,
    }
  }
}

impl<'a> ObjectProperty<'a> {
  pub fn get(
    &mut self,
    analyzer: &Analyzer<'a>,
    values: &mut Vec<Entity<'a>>,
    getters: &mut Vec<Entity<'a>>,
    non_existent: &mut ConsumableVec<'a>,
  ) {
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => values.push(*value),
        ObjectPropertyValue::Property(Some(getter), _) => getters.push(*getter),
        ObjectPropertyValue::Property(None, _) => values.push(analyzer.factory.undefined),
      }
    }

    if let Some(dep) = self.non_existent.try_collect(analyzer.factory) {
      non_existent.push(dep);
    } else if !self.definite && non_existent.is_empty() {
      non_existent.push(analyzer.factory.empty_consumable);
    }
  }

  pub fn get_mangable(
    &mut self,
    analyzer: &Analyzer<'a>,
    values: &mut Vec<Entity<'a>>,
    getters: &mut Vec<Entity<'a>>,
    non_existent: &mut ConsumableVec<'a>,
    key: Entity<'a>,
    key_atom: MangleAtom,
  ) {
    let (prev_key, prev_atom) = self.mangling.unwrap();
    let constraint = &*analyzer.factory.alloc(MangleConstraint::Eq(prev_atom, key_atom));
    for possible_value in &self.possible_values {
      match possible_value {
        ObjectPropertyValue::Field(value, _) => {
          values.push(analyzer.factory.mangable(*value, (prev_key, key), constraint))
        }
        ObjectPropertyValue::Property(Some(getter), _) => {
          getters.push(analyzer.factory.mangable(*getter, (prev_key, key), constraint))
        }
        ObjectPropertyValue::Property(None, _) => values.push(analyzer.factory.mangable(
          analyzer.factory.undefined,
          (prev_key, key),
          constraint,
        )),
      }
    }

    if let Some(dep) = self.non_existent.try_collect(analyzer.factory) {
      non_existent.push(dep);
    } else if !self.definite && non_existent.is_empty() {
      non_existent.push(analyzer.factory.empty_consumable);
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

    if let Some((key, _)) = self.mangling {
      analyzer.consume(key);
    }
  }
}
