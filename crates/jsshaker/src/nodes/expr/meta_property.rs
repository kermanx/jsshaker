use oxc::ast::ast::{Expression, MetaProperty};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_meta_property(&mut self, node: &'a MetaProperty<'a>) -> Entity<'a> {
    let meta = node.meta.name.as_str();
    let property = node.property.name.as_str();

    if meta == "import" && property == "meta" {
      self.builtins.import_meta
    } else {
      self.factory.unknown
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_meta_property(
    &self,
    node: &'a MetaProperty<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let MetaProperty { span, meta, property } = node;

    need_val.then(|| self.ast.expression_meta_property(*span, meta.clone(), property.clone()))
  }
}
