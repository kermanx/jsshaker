use oxc::{
  ast::ast::{
    ComputedMemberExpression, Expression, MemberExpression, PrivateFieldExpression,
    StaticMemberExpression,
  },
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, transformer::Transformer,
};

pub type MemberExpressionReadResult<'a> =
  Result<(usize, Entity<'a>, Option<Entity<'a>>, (Entity<'a>, Entity<'a>)), Entity<'a>>;

impl<'a> Analyzer<'a> {
  /// Returns (short-circuit, value, cache)
  pub fn exec_member_expression_read(
    &mut self,
    node: &'a MemberExpression<'a>,
    will_write: bool,
  ) -> (Entity<'a>, (Entity<'a>, Entity<'a>)) {
    let (scope_count, value, undefined, cache) =
      self.exec_member_expression_read_in_chain(node, will_write).unwrap();

    assert_eq!(scope_count, 0);
    assert!(undefined.is_none());

    (value, cache)
  }

  /// Returns (scope_count, value, forwarded_undefined, cache)
  pub fn exec_member_expression_read_in_chain(
    &mut self,
    node: &'a MemberExpression<'a>,
    will_write: bool,
  ) -> MemberExpressionReadResult<'a> {
    let (mut scope_count, object, mut undefined) = self.exec_expression_in_chain(node.object())?;

    let dep_id = AstKind2::MemberExpression(node);

    if node.optional() {
      let maybe_left = match object.test_nullish() {
        Some(true) => {
          self.pop_multiple_cf_scopes(scope_count);
          return Err(self.forward_logical_left_val(dep_id, self.factory.undefined, true, false));
        }
        Some(false) => false,
        None => {
          undefined = Some(self.forward_logical_left_val(
            dep_id,
            undefined.unwrap_or(self.factory.undefined),
            true,
            false,
          ));
          true
        }
      };

      self.push_logical_right_cf_scope(dep_id, object, maybe_left, true);
      scope_count += 1;
    }

    if will_write {
      self.push_dependent_cf_scope(object);
    }
    let key = self.exec_key(node);
    if will_write {
      self.pop_cf_scope();
    }

    let value = object.get_property(self, dep_id, key);

    Ok((scope_count, value, undefined, (object, key)))
  }

  pub fn exec_member_expression_write(
    &mut self,
    node: &'a MemberExpression<'a>,
    value: Entity<'a>,
    cache: Option<(Entity<'a>, Entity<'a>)>,
  ) {
    let (object, key) = cache.unwrap_or_else(|| {
      let object = self.exec_expression(node.object());

      self.push_dependent_cf_scope(object);
      let key = self.exec_key(node);
      self.pop_cf_scope();

      (object, key)
    });

    object.set_property(self, AstKind2::MemberExpression(node), key, value);
  }

  fn exec_key(&mut self, node: &'a MemberExpression<'a>) -> Entity<'a> {
    match node {
      MemberExpression::ComputedMemberExpression(node) => {
        self.exec_expression(&node.expression).get_to_property_key(self)
      }
      MemberExpression::StaticMemberExpression(node) => self.exec_identifier_name(&node.property),
      MemberExpression::PrivateFieldExpression(node) => self.exec_private_identifier(&node.field),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_member_expression_read(
    &self,
    node: &'a MemberExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    self.transform_member_expression_read_in_chain(node, need_val).unwrap()
  }

  pub fn transform_member_expression_read_in_chain(
    &self,
    node: &'a MemberExpression<'a>,
    need_val: bool,
  ) -> Result<Option<Expression<'a>>, Option<Expression<'a>>> {
    let dep_id = AstKind2::MemberExpression(node);

    let (need_optional, must_short_circuit) = self.get_chain_result(dep_id, node.optional());

    if must_short_circuit {
      let object = self.transform_expression_in_chain(node.object(), false)?;
      return Err(object);
    }

    let need_read = need_val || self.is_referred(dep_id);

    if !need_read {
      let object = self.transform_expression_in_chain(node.object(), need_optional)?;
      let key_effect = match node {
        MemberExpression::ComputedMemberExpression(node) => {
          self.transform_expression(&node.expression, false)
        }
        _ => None,
      };

      return Ok(if need_optional {
        Some(self.build_chain_expression_mock(node.span(), object.unwrap(), key_effect.unwrap()))
      } else {
        build_effect!(&self.ast_builder, node.span(), object, key_effect)
      });
    }

    Ok(match node {
      MemberExpression::ComputedMemberExpression(node) => {
        let ComputedMemberExpression { span, object, expression, .. } = node.as_ref();

        let object = self.transform_expression_in_chain(object, need_read)?.unwrap();
        let key = self.transform_expression(expression, need_read);

        if need_read {
          Some(Expression::from(self.ast_builder.member_expression_computed(
            *span,
            object,
            key.unwrap(),
            need_optional,
          )))
        } else {
          build_effect!(&self.ast_builder, *span, object, key)
        }
      }
      MemberExpression::StaticMemberExpression(node) => {
        let StaticMemberExpression { span, object, property, .. } = node.as_ref();

        let object = self.transform_expression(object, need_read);

        if need_read {
          Some(Expression::from(self.ast_builder.member_expression_static(
            *span,
            object.unwrap(),
            self.transform_identifier_name(property),
            need_optional,
          )))
        } else {
          object
        }
      }
      MemberExpression::PrivateFieldExpression(node) => {
        let PrivateFieldExpression { span, object, field, .. } = node.as_ref();

        let object = self.transform_expression(object, need_read);

        if need_read {
          Some(
            self
              .ast_builder
              .member_expression_private_field_expression(
                *span,
                object.unwrap(),
                self.transform_private_identifier(field),
                need_optional,
              )
              .into(),
          )
        } else {
          object
        }
      }
    })
  }

  pub fn transform_member_expression_write(
    &self,
    node: &'a MemberExpression<'a>,
  ) -> Option<MemberExpression<'a>> {
    let need_write = self.is_referred(AstKind2::MemberExpression(node));

    match node {
      MemberExpression::ComputedMemberExpression(node) => {
        let ComputedMemberExpression { span, object, expression, .. } = node.as_ref();

        let transformed_object = self.transform_expression(object, need_write);

        let need_key_value = need_write || transformed_object.is_some();
        let transformed_key = self.transform_expression(expression, need_key_value);

        if need_key_value {
          Some(self.ast_builder.member_expression_computed(
            *span,
            transformed_object.unwrap(),
            transformed_key.unwrap(),
            false,
          ))
        } else if transformed_key.is_some() {
          Some(self.ast_builder.member_expression_computed(
            *span,
            self.transform_expression(object, true).unwrap(),
            self.transform_expression(expression, true).unwrap(),
            false,
          ))
        } else {
          None
        }
      }
      MemberExpression::StaticMemberExpression(node) => {
        let StaticMemberExpression { span, object, property, .. } = node.as_ref();

        let transformed_object = self.transform_expression(object, need_write);
        let property = self.transform_identifier_name(property);

        if need_write {
          Some(self.ast_builder.member_expression_static(
            *span,
            transformed_object.unwrap(),
            property,
            false,
          ))
        } else if transformed_object.is_some() {
          Some(self.ast_builder.member_expression_static(
            *span,
            self.transform_expression(object, true).unwrap(),
            property,
            false,
          ))
        } else {
          None
        }
      }
      MemberExpression::PrivateFieldExpression(node) => {
        let PrivateFieldExpression { span, object, field, .. } = node.as_ref();

        let transformed_object = self.transform_expression(object, need_write);
        let field = self.transform_private_identifier(field);

        if need_write {
          Some(self.ast_builder.member_expression_private_field_expression(
            *span,
            transformed_object.unwrap(),
            field,
            false,
          ))
        } else if transformed_object.is_some() {
          Some(self.ast_builder.member_expression_private_field_expression(
            *span,
            self.transform_expression(object, true).unwrap(),
            field,
            false,
          ))
        } else {
          None
        }
      }
    }
  }
}
