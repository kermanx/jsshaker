use super::ObjectValue;
use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep},
  entity::Entity,
  mangling::{MangleConstraint, ManglingDep},
  value::escaped,
};

impl<'a> ObjectValue<'a> {
  pub fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    if self.is_self_or_proto_consumed() {
      return escaped::delete_property(analyzer, dep, key);
    }

    let (_target_depth, is_exhaustive, non_det, deps) = self.prepare_mutation(analyzer, dep);

    if is_exhaustive {
      self.consume(analyzer);
      return escaped::delete_property(analyzer, dep, key);
    }

    let deps = analyzer.dep(deps);
    {
      let mut unknown_keyed = self.unknown.borrow_mut();
      if !unknown_keyed.possible_values.is_empty() {
        unknown_keyed.delete(true, analyzer.dep((deps, key)));
      }
    }

    if let Some(key_literals) = key.get_literals(analyzer) {
      let non_det = non_det || key_literals.len() > 1;
      let mangable = self.check_mangable(analyzer, &key_literals);
      let deps = if mangable { deps } else { analyzer.dep((deps, key)) };

      let mut string_keyed = self.keyed.borrow_mut();
      for &key_literal in &key_literals {
        let (key_str, key_atom) = key_literal.into();
        if let Some(property) = string_keyed.get_mut(&key_str) {
          property.delete(
            non_det,
            if mangable {
              let prev_key = property.key.unwrap();
              let prev_atom = property.mangling.unwrap();
              analyzer.dep((
                deps,
                ManglingDep {
                  deps: (prev_key, key),
                  constraint: MangleConstraint::Eq(prev_atom, key_atom.unwrap()),
                },
              ))
            } else {
              deps
            },
          );
        } else if let Some(rest) = &self.rest {
          rest.borrow_mut().delete(true, analyzer.dep((deps, key)));
        } else if mangable {
          self.add_to_mangling_group(analyzer, key_atom.unwrap());
        }
      }
    } else {
      self.disable_mangling(analyzer);

      let deps = analyzer.dep((deps, key));

      let mut string_keyed = self.keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.delete(true, deps);
      }
    }
  }
}
