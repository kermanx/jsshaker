use crate::{
  analyzer::Analyzer,
  entity::{Entity, ObjectPrototype},
  utils::dep_id::DepId,
};

impl<'a> Analyzer<'a> {
  /// const { enumerated_1, enumerated_2, ...rest } = object;
  pub fn exec_object_rest(
    &mut self,
    dep: impl Into<DepId>,
    object: Entity<'a>,
    enumerated: Vec<Entity<'a>>,
  ) -> Entity<'a> {
    let rest = self.new_empty_object(ObjectPrototype::ImplicitOrNull, None);
    rest.init_spread(self, dep.into(), object);
    let rest = Entity::from(rest);
    for key in enumerated {
      rest.delete_property(self, self.factory.no_dep, key);
    }
    rest
  }
}
