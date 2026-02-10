use oxc::{allocator::Allocator, span::Atom};
use oxc_ecmascript::{StringCharAt, StringCharAtResult, StringToNumber};

use super::{LiteralValue, PossibleLiterals};
use crate::{
  analyzer::Analyzer,
  dep::Dep,
  entity::Entity,
  value::{
    ArgumentsValue, EnumeratedProperties, IteratedElements, PropertyKeyValue, TypeofResult,
    ValueTrait, cacheable::Cacheable, escaped,
  },
};

pub trait ToAtomRef<'a> {
  fn to_atom_ref(self, allocator: &'a Allocator) -> &'a Atom<'a>;
}
impl<'a> ToAtomRef<'a> for &'a Atom<'a> {
  fn to_atom_ref(self, _allocator: &'a Allocator) -> &'a Atom<'a> {
    self
  }
}
impl<'a> ToAtomRef<'a> for &'a str {
  fn to_atom_ref(self, allocator: &'a Allocator) -> &'a Atom<'a> {
    allocator.alloc(Atom::from(self))
  }
}
impl<'a> ToAtomRef<'a> for String {
  fn to_atom_ref(self, allocator: &'a Allocator) -> &'a Atom<'a> {
    allocator.alloc_str(&self).to_atom_ref(allocator)
  }
}

impl<'a> ValueTrait<'a> for Atom<'a> {
  fn include(&self, _analyzer: &mut Analyzer<'a>) {
    // No effect
  }

  fn include_mangable(&self, _analyzer: &mut Analyzer<'a>) -> bool {
    // No effect
    true
  }

  fn unknown_mutate(&self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    let prototype = &analyzer.builtins.prototypes.string;
    let dep = analyzer.dep((dep, key));
    if let Some(key_literals) = key.get_literals(analyzer) {
      let mut values = analyzer.factory.vec();
      for &key_literal in &key_literals {
        if let Some(property) = get_known_instance_property(self.as_str(), analyzer, key_literal) {
          values.push(property);
        } else if let Some(property) = prototype.get_literal_keyed(key_literal) {
          values.push(property);
        } else {
          values.push(analyzer.factory.unmatched_prototype_property);
        }
      }
      analyzer.factory.computed_union(values, dep)
    } else {
      analyzer.factory.computed_unknown(dep)
    }
  }

  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
    // No effect
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    let value = self.as_str();
    if value.len() < analyzer.config.max_folding_string_length {
      EnumeratedProperties {
        known: value
          .char_indices()
          .map(|(i, c)| {
            let i_str = i.to_string().to_atom_ref(analyzer.allocator);
            let c_str = c.to_string().to_atom_ref(analyzer.allocator);
            (
              PropertyKeyValue::String(i_str),
              (
                true,
                analyzer.factory.unmangable_string(i_str),
                analyzer.factory.unmangable_string(c_str),
              ),
            )
          })
          .collect(),
        unknown: None,
        dep,
      }
    } else {
      analyzer.factory.computed_unknown_string(self).enumerate_properties(analyzer, dep)
    }
  }

  fn delete_property(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>, _key: Entity<'a>) {
    // No effect
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot call a non-function object {:?}", self));
    if analyzer.config.preserve_exceptions {
      escaped::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot construct a non-constructor object {:?}", self));
    if analyzer.config.preserve_exceptions {
      escaped::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, attributes: Entity<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown((self, attributes))
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    analyzer.factory.computed(self.into(), dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    let value = self.as_str();
    (vec![], (!value.is_empty()).then_some(analyzer.factory.unknown_string), dep)
  }

  fn coerce_string(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let value = self.as_str();
    let val = value.trim().string_to_number();
    if val.is_nan() { analyzer.factory.nan } else { analyzer.factory.number(val) }
  }

  fn coerce_primitive(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(!self.as_str().is_empty())
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.coerce_string(analyzer)
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.coerce_string(analyzer)
  }

  fn get_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<PossibleLiterals<'a>> {
    Some(PossibleLiterals::Single(LiteralValue::String(self, None)))
  }

  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    Some(LiteralValue::String(self, None))
  }

  fn get_keys(
    &self,
    _analyzer: &Analyzer<'a>,
    _check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    None
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::String
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(!self.as_str().is_empty())
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::String(self.as_str()))
  }
}

fn get_known_instance_property<'a>(
  value: &str,
  analyzer: &Analyzer<'a>,
  key: LiteralValue<'a>,
) -> Option<Entity<'a>> {
  let LiteralValue::String(key, _) = key else { return None };
  if key == "length" {
    Some(analyzer.factory.number(value.len() as f64))
  } else {
    let index = key.as_str().string_to_number();
    if index.is_finite() {
      Some(match value.char_at(Some(index)) {
        StringCharAtResult::InvalidChar(_) => analyzer.factory.unknown,
        StringCharAtResult::OutOfRange => analyzer.factory.undefined,
        StringCharAtResult::Value(c) => analyzer.factory.unmangable_string(c.to_string()),
      })
    } else {
      None
    }
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  pub fn mangable_string(
    &self,
    value: impl ToAtomRef<'a>,
    atom: crate::mangling::MangleAtom,
  ) -> Entity<'a> {
    self.string(value, Some(atom))
  }

  pub fn unmangable_string(&self, value: impl ToAtomRef<'a>) -> Entity<'a> {
    self.string(value, None)
  }

  pub fn string(
    &self,
    value: impl ToAtomRef<'a>,
    atom: Option<crate::mangling::MangleAtom>,
  ) -> Entity<'a> {
    if atom.is_some() {
      self.alloc(LiteralValue::String(value.to_atom_ref(self.allocator), atom)).into()
    } else {
      value.to_atom_ref(self.allocator).into()
    }
  }
}
