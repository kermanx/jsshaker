use super::{ArgumentsValue, EnumeratedProperties, IteratedElements, Value};
use crate::{analyzer::Analyzer, dep::Dep, entity::Entity};

pub fn unknown_mutate<'a>(analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
  analyzer.global_effect();
  analyzer.include(dep);
}

pub fn get_property<'a>(
  target: Value<'a>,
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  key: Entity<'a>,
) -> Entity<'a> {
  if analyzer.config.unknown_property_read_side_effects {
    analyzer.include((target, dep, key));
    analyzer.global_effect();
    analyzer.factory.unknown
  } else {
    analyzer.factory.computed_unknown((target, dep, key))
  }
}

pub fn set_property<'a>(
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  key: Entity<'a>,
  value: Entity<'a>,
) {
  analyzer.global_effect();
  analyzer.include((dep, key, value));
}

pub fn enumerate_properties<'a>(
  target: Value<'a>,
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
) -> EnumeratedProperties<'a> {
  EnumeratedProperties {
    known: Default::default(),
    unknown: Some(analyzer.factory.unknown),
    dep: if analyzer.config.unknown_property_read_side_effects {
      analyzer.include(dep);
      analyzer.global_effect();
      analyzer.factory.no_dep
    } else {
      analyzer.dep((target, dep))
    },
  }
}

pub fn delete_property<'a>(analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
  analyzer.global_effect();
  analyzer.include((dep, key));
}

pub fn call<'a>(
  target: Value<'a>,
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  this: Entity<'a>,
  args: ArgumentsValue<'a>,
) -> Entity<'a> {
  analyzer.include((target, dep, this, args));
  analyzer.global_effect();
  analyzer.factory.unknown
}

pub fn builtin_call<'a>(
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  this: Entity<'a>,
  args: ArgumentsValue<'a>,
) -> Entity<'a> {
  analyzer.include((dep, this, args));
  analyzer.global_effect();
  analyzer.factory.unknown
}

pub fn construct<'a>(
  target: Value<'a>,
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  args: ArgumentsValue<'a>,
) -> Entity<'a> {
  analyzer.include((target, dep, args));
  analyzer.global_effect();
  analyzer.factory.unknown
}

pub fn jsx<'a>(target: Value<'a>, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
  // No include!
  analyzer.factory.computed_unknown((target, props))
}

pub fn r#await<'a>(analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
  analyzer.include(dep);
  analyzer.global_effect();
  analyzer.factory.unknown
}

pub fn iterate<'a>(analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
  if analyzer.config.iterate_side_effects {
    analyzer.include(dep);
    analyzer.global_effect();
    (vec![], Some(analyzer.factory.unknown), analyzer.factory.no_dep)
  } else {
    (vec![], Some(analyzer.factory.unknown), dep)
  }
}

pub fn coerce_string<'a>(analyzer: &Analyzer<'a>) -> Entity<'a> {
  analyzer.factory.unknown_string
}

pub fn coerce_numeric<'a>(analyzer: &Analyzer<'a>) -> Entity<'a> {
  // Possibly number or bigint
  analyzer.factory.unknown
}
