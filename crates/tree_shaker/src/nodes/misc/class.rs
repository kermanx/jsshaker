use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  consumable::ConsumableTrait,
  entity::{ClassEntity, Entity},
  transformer::Transformer,
  utils::CalleeNode,
};
use oxc::{
  allocator,
  ast::{
    ast::{
      Class, ClassBody, ClassElement, ClassType, MethodDefinitionKind, PropertyDefinitionType,
      PropertyKind, StaticBlock,
    },
    NONE,
  },
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn exec_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let super_class = node.super_class.as_ref().map(|node| self.exec_expression(node));

    let mut keys = vec![];
    for element in &node.body.body {
      keys.push(element.property_key().map(|key| self.exec_property_key(key)));
    }

    self.push_variable_scope();

    let statics = self.new_empty_object(&self.builtins.prototypes.function, None);
    for (index, element) in node.body.body.iter().enumerate() {
      if let ClassElement::MethodDefinition(node) = element {
        if node.r#static {
          let key = keys[index].unwrap();
          let value = self.exec_function(&node.value);
          let kind = match node.kind {
            MethodDefinitionKind::Constructor => unreachable!(),
            MethodDefinitionKind::Method => PropertyKind::Init,
            MethodDefinitionKind::Get => PropertyKind::Get,
            MethodDefinitionKind::Set => PropertyKind::Set,
          };
          statics.init_property(self, kind, key, value, true);
        }
      }
    }

    self.pop_variable_scope();

    let class = self.factory.class(
      self.current_module(),
      node,
      keys.clone(),
      self.scope_context.variable.stack.clone(),
      super_class,
      statics,
    );

    let variable_scope_stack = self.scope_context.variable.stack.clone();
    self.push_call_scope(
      self.new_callee_info(CalleeNode::ClassStatics(node)),
      self.factory.empty_consumable,
      variable_scope_stack,
      false,
      false,
      false,
    );

    let variable_scope = self.variable_scope_mut();
    variable_scope.this = Some(class);

    if let Some(id) = &node.id {
      self.declare_binding_identifier(id, false, DeclarationKind::NamedFunctionInBody);
      self.init_binding_identifier(id, Some(class));
    }

    for (index, element) in node.body.body.iter().enumerate() {
      match element {
        ClassElement::StaticBlock(node) => self.exec_static_block(node),
        ClassElement::MethodDefinition(_node) => {}
        ClassElement::PropertyDefinition(node) if node.r#static => {
          if let Some(value) = &node.value {
            let key = keys[index].unwrap();
            let value = self.exec_expression(value);
            class.set_property(
              self,
              self.consumable(AstKind2::PropertyDefinition(node)),
              key,
              value,
            );
          }
        }
        _ => {}
      }
    }

    self.pop_call_scope();

    class
  }

  pub fn declare_class(&mut self, node: &'a Class<'a>, exporting: bool) {
    self.declare_binding_identifier(node.id.as_ref().unwrap(), exporting, DeclarationKind::Class);
  }

  pub fn init_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let value = self.exec_class(node);

    self.init_binding_identifier(node.id.as_ref().unwrap(), Some(value));

    value
  }

  pub fn construct_class(&mut self, class: &ClassEntity<'a>) {
    self.module_stack.push(class.module);
    let node = class.node;

    self.consume(AstKind2::Class(node));

    class.super_class.consume(self);

    // Keys
    for (index, element) in node.body.body.iter().enumerate() {
      if !element.r#static() {
        if let Some(key) = class.keys[index] {
          key.consume(self);
        }
      }
    }

    if let Some(id) = &node.id {
      self.push_variable_scope();
      self.declare_binding_identifier(id, false, DeclarationKind::NamedFunctionInBody);
      self.init_binding_identifier(id, Some(self.factory.unknown()));
    }

    // Non-static methods
    self.push_call_scope(
      self.new_callee_info(CalleeNode::ClassConstructor(node)),
      self.factory.empty_consumable,
      class.variable_scope_stack.as_ref().clone(),
      false,
      false,
      false,
    );
    for element in &node.body.body {
      if let ClassElement::MethodDefinition(node) = element {
        if !node.r#static {
          let value = self.exec_function(&node.value);
          self.consume(value);
        }
      }
    }
    self.pop_call_scope();

    // Non-static properties
    let variable_scope_stack = class.variable_scope_stack.clone();
    self.exec_consumed_fn("class_property", move |analyzer| {
      analyzer.push_call_scope(
        analyzer.new_callee_info(CalleeNode::ClassConstructor(node)),
        analyzer.factory.empty_consumable,
        variable_scope_stack.as_ref().clone(),
        false,
        false,
        false,
      );

      let this = analyzer.factory.unknown();
      let arguments = analyzer.factory.immutable_unknown;
      let variable_scope = analyzer.variable_scope_mut();
      variable_scope.this = Some(this);
      variable_scope.arguments = Some((arguments, vec![]));

      for element in &node.body.body {
        if let ClassElement::PropertyDefinition(node) = element {
          if !node.r#static {
            if let Some(value) = &node.value {
              let value = analyzer.exec_expression(value);
              analyzer.consume(value);
            }
          }
        }
      }

      analyzer.pop_call_scope();

      analyzer.factory.undefined
    });

    if node.id.is_some() {
      self.pop_variable_scope();
    }
    self.module_stack.pop();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_class(
    &self,
    node: &'a Class<'a>,
    need_val: bool,
  ) -> Option<allocator::Box<'a, Class<'a>>> {
    let Class { r#type, span, id, super_class, body, .. } = node;

    let transformed_id = id.as_ref().and_then(|node| self.transform_binding_identifier(node));

    if need_val || transformed_id.is_some() {
      let id = if self.config.preserve_function_name {
        self.clone_node(id)
      } else if node.r#type == ClassType::ClassDeclaration && id.is_some() {
        // Id cannot be omitted for class declaration
        // However, we still check `id.is_some()` to handle `export default class {}`
        Some(
          transformed_id
            .unwrap_or_else(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
        )
      } else {
        transformed_id
      };

      let ever_constructed = self.is_referred(AstKind2::Class(node));

      let super_class = super_class.as_ref().and_then(|node| {
        if ever_constructed || self.transform_expression(node, false).is_some() {
          self.transform_expression(node, true)
        } else {
          None
        }
      });

      let body = {
        let ClassBody { span, body } = body.as_ref();

        let mut transformed_body = self.ast_builder.vec();

        for element in body {
          if ever_constructed || element.r#static() {
            if let Some(element) = match element {
              ClassElement::StaticBlock(node) => {
                self.transform_static_block(node).map(ClassElement::StaticBlock)
              }
              ClassElement::MethodDefinition(node) => self.transform_method_definition(node),
              ClassElement::PropertyDefinition(node) => self.transform_property_definition(node),
              ClassElement::AccessorProperty(_node) => unreachable!(),
              ClassElement::TSIndexSignature(_node) => unreachable!(),
            } {
              transformed_body.push(element);
            }
          } else if let Some(key) =
            element.property_key().and_then(|key| self.transform_property_key(key, false))
          {
            transformed_body.push(self.ast_builder.class_element_property_definition(
              element.span(),
              PropertyDefinitionType::PropertyDefinition,
              self.ast_builder.vec(),
              key,
              None,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              NONE,
              None,
            ));
          }
        }

        self.ast_builder.class_body(*span, transformed_body)
      };

      Some(self.ast_builder.alloc_class(
        *span,
        *r#type,
        self.ast_builder.vec(),
        id,
        NONE,
        super_class,
        NONE,
        None,
        body,
        false,
        false,
      ))
    } else {
      let mut statements = self.ast_builder.vec();

      if let Some(super_class) = super_class {
        let span = super_class.span();
        if let Some(super_class) = self.transform_expression(super_class, false) {
          statements.push(self.ast_builder.statement_expression(span, super_class));
        }
      }

      for element in &body.body {
        if let Some(key) = element.property_key() {
          if key.is_expression() {
            if let Some(element) = self.transform_expression(key.to_expression(), false) {
              statements.push(self.ast_builder.statement_expression(element.span(), element));
            }
          }
        }
      }

      for element in &body.body {
        match element {
          ClassElement::StaticBlock(node) => {
            if let Some(node) = self.transform_static_block(node) {
              let StaticBlock { span, body, .. } = node.unbox();
              statements.push(self.ast_builder.statement_block(span, body));
            }
          }
          ClassElement::PropertyDefinition(node) if node.r#static => {
            if let Some(value) = &node.value {
              let span = value.span();
              if let Some(value) = self.transform_expression(value, false) {
                statements.push(self.ast_builder.statement_expression(span, value));
              }
            }
          }
          _ => {}
        }
      }

      if statements.is_empty() {
        None
      } else {
        Some(
          self.ast_builder.alloc_class(
            *span,
            *r#type,
            self.ast_builder.vec(),
            (node.r#type == ClassType::ClassDeclaration)
              .then(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
            NONE,
            None,
            NONE,
            None,
            self.ast_builder.class_body(
              body.span(),
              self
                .ast_builder
                .vec1(self.ast_builder.class_element_static_block(body.span(), statements)),
            ),
            false,
            false,
          ),
        )
      }
    }
  }
}
