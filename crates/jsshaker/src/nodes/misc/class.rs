use oxc::{
  allocator::{self, Allocator},
  ast::{
    NONE,
    ast::{
      Class, ClassBody, ClassElement, ClassType, MethodDefinition, MethodDefinitionKind,
      PropertyDefinitionType, PropertyKind, StaticBlock,
    },
  },
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  dep::DepAtom,
  entity::Entity,
  transformer::Transformer,
  utils::{CalleeNode, DefaultIn},
  value::{ObjectPrototype, cache::FnCacheTrackingData, call::FnCallInfo},
};

struct Data<'a> {
  pub value: Option<Entity<'a>>,
  pub constructor: Option<&'a MethodDefinition<'a>>,
  pub keys: allocator::Vec<'a, Option<Entity<'a>>>,
  pub super_class: Option<Entity<'a>>,
}

impl<'a> DefaultIn<'a> for Data<'a> {
  fn default_in(allocator: &'a Allocator) -> Self {
    Data {
      value: None,
      constructor: None,
      keys: allocator::Vec::new_in(allocator),
      super_class: None,
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn exec_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let data = self.load_data::<Data>(AstKind2::Class(node));
    let (class, prototype) = self.new_function_with_prototype(CalleeNode::ClassConstructor(node));

    // 1. Execute super class
    data.super_class = node.super_class.as_ref().map(|node| self.exec_expression(node));
    if let Some(super_class) = &data.super_class {
      // Because we can't re-define the "prototype" property, this should be side-effect free
      if let Some((prototype_dep, super_statics, super_prototype)) =
        super_class.get_constructor_prototype(self, self.factory.no_dep)
      {
        class.statics.set_prototype(super_statics);
        prototype.set_prototype(super_prototype);
        prototype.add_extra_dep(prototype_dep);
      } else {
        let dep = self.factory.dep(*super_class);
        class.statics.set_prototype(ObjectPrototype::Unknown(dep));
        prototype.set_prototype(ObjectPrototype::Unknown(dep));
      }
    } else {
      prototype.set_prototype(ObjectPrototype::ImplicitOrNull);
    };

    // Enter class statics scope
    self.variable_scope_mut().super_class =
      Some(data.super_class.unwrap_or(self.factory.undefined));

    // 2. Execute keys and find constructor
    for element in &node.body.body {
      let key = element.property_key().map(|key| self.exec_property_key(key, None));
      data.keys.push(key);

      if let ClassElement::MethodDefinition(method) = element
        && method.kind.is_constructor()
      {
        if data.constructor.is_some() {
          self.throw_builtin_error("A class may only have one constructor");
        }
        data.constructor = Some(method);
      }
    }

    // 3. Register methods
    for (key, element) in data.keys.iter().zip(node.body.body.iter()) {
      if let ClassElement::MethodDefinition(node) = element {
        let kind = match node.kind {
          MethodDefinitionKind::Constructor => continue,
          MethodDefinitionKind::Method => PropertyKind::Init,
          MethodDefinitionKind::Get => PropertyKind::Get,
          MethodDefinitionKind::Set => PropertyKind::Set,
        };
        let value = self.exec_function(&node.value);
        if node.r#static {
          class.statics.init_property(self, kind, key.unwrap(), value, true);
        } else {
          prototype.init_property(self, kind, key.unwrap(), value, true);
        }
      }
    }

    // 3.5. Execute member decorators (ES2025 Stage 3)
    for (index, element) in node.body.body.iter().enumerate() {
      match element {
        ClassElement::MethodDefinition(method) if !method.decorators.is_empty() => {
          // Skip constructor decorators (not allowed in ES2025 Stage 3)
          if method.kind.is_constructor() {
            continue;
          }

          let key = data.keys[index].unwrap();
          let is_static = method.r#static;
          let target = if is_static { class.statics } else { prototype };

          // Get the original method value
          let this: crate::value::Value<'a> = if is_static { class } else { prototype };
          let original_value = target.get_property(self, this, self.factory.no_dep, key);

          // Execute decorators
          let decorated_value = self.exec_decorators(&method.decorators, original_value);

          // Update the method value
          target.set_property(self, self.factory.no_dep, key, decorated_value);
        }
        ClassElement::PropertyDefinition(prop) if !prop.decorators.is_empty() => {
          // Field decorators receive undefined as the value
          let _decorated_initializer =
            self.exec_decorators(&prop.decorators, self.factory.undefined);

          // TODO: Store decorated initializer for later execution
          // For now, we just track that the decorator was executed
        }
        _ => {}
      }
    }

    // 4. Execute static blocks
    if let Some(id) = &node.id {
      self.declare_binding_identifier(id, None, DeclarationKind::NamedFunctionInBody);
      self.init_binding_identifier(id, DeclarationKind::NamedFunctionInBody, Some(class.into()));
    }

    for (index, element) in node.body.body.iter().enumerate() {
      match element {
        ClassElement::StaticBlock(node) => self.exec_static_block(node),
        ClassElement::MethodDefinition(_node) => {}
        ClassElement::PropertyDefinition(node) if node.r#static => {
          let key = data.keys[index].unwrap();
          let value = self.exec_property_definition(node);
          class.statics.init_property(self, PropertyKind::Init, key, value, true);
        }
        _ => {}
      }
    }

    // 5. Execute class decorators (ES2025 Stage 3)
    let class = class.into();
    let final_class = if !node.decorators.is_empty() {
      self.exec_decorators(&node.decorators, class)
    } else {
      class
    };

    data.value = Some(final_class);
    final_class
  }

  pub fn declare_class(&mut self, node: &'a Class<'a>, exporting: Option<DepAtom>) {
    self.declare_binding_identifier(node.id.as_ref().unwrap(), exporting, DeclarationKind::Class);
  }

  pub fn init_class(&mut self, node: &'a Class<'a>) -> Entity<'a> {
    let value = self.exec_class(node);

    self.init_binding_identifier(node.id.as_ref().unwrap(), DeclarationKind::Class, Some(value));

    value
  }

  pub fn call_class_constructor(
    &mut self,
    node: &'a Class<'a>,
    info: FnCallInfo<'a>,
  ) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    let factory = self.factory;
    let data = self.load_data::<Data>(AstKind2::Class(node));

    self.push_call_scope(info, false, false);
    let super_class = data.super_class.unwrap_or(self.factory.undefined);
    let variable_scope = self.variable_scope_mut();
    variable_scope.this = Some(info.this);
    variable_scope.arguments =
      Some((info.args, factory.vec(/* later filled by formal parameters */)));
    variable_scope.super_class = Some(super_class);

    if let Some(id) = &node.id {
      self.declare_binding_identifier(id, None, DeclarationKind::NamedFunctionInBody);
      self.init_binding_identifier(id, DeclarationKind::NamedFunctionInBody, data.value);
    }

    // 1. Init properties
    for (key, element) in data.keys.iter().zip(node.body.body.iter()) {
      if let ClassElement::PropertyDefinition(node) = element
        && !node.r#static
      {
        let value = self.exec_property_definition(node);
        info.this.set_property(self, self.factory.no_dep, key.unwrap(), value);
      }
    }

    // 2. Call constructor
    if let Some(constructor) = data.constructor {
      let function = constructor.value.as_ref();
      let dep = self.factory.dep(AstKind2::Function(function));
      self.cf_scope_mut().push_dep(dep);
      self.exec_formal_parameters(&function.params, info.args, DeclarationKind::FunctionParameter);
      self.exec_function_body(function.body.as_ref().unwrap());
      if info.include {
        self.include_return_values();
      }
      let (ret_val, _) = self.pop_call_scope();
      let ret_val = self.factory.computed(ret_val, dep);
      (ret_val, FnCacheTrackingData::worst_case())
    } else if let Some(super_class) = &data.super_class {
      self.pop_call_scope();
      let ret_val = super_class.call(self, self.factory.no_dep, info.this, info.args);
      (ret_val, FnCacheTrackingData::worst_case())
    } else {
      let (_, cache_tracking) = self.pop_call_scope();
      (self.factory.undefined, cache_tracking)
    }
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
        // `id` cannot be omitted for class declaration
        // However, we still check `id.is_some()` to handle `export default class {}`
        Some(
          transformed_id
            .unwrap_or_else(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
        )
      } else {
        transformed_id
      };

      let super_class = super_class.as_ref().and_then(|node| self.transform_expression(node, true));

      let body = {
        let ClassBody { span, body } = body.as_ref();

        let mut transformed_body = self.ast.vec();

        for element in body {
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
          } else if let Some(key) =
            element.property_key().and_then(|key| self.transform_property_key(key, false))
          {
            transformed_body.push(self.ast.class_element_property_definition(
              element.span(),
              PropertyDefinitionType::PropertyDefinition,
              self.ast.vec(),
              key,
              NONE,
              None,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              None,
            ));
          }
        }

        self.ast.class_body(*span, transformed_body)
      };

      let decorators = self.transform_decorators(&node.decorators);

      Some(self.ast.alloc_class(
        *span,
        *r#type,
        decorators,
        id,
        NONE,
        super_class,
        NONE,
        self.ast.vec(),
        body,
        false,
        false,
      ))
    } else {
      // Side-effect only

      let mut statements = self.ast.vec();

      if let Some(super_class) = super_class {
        let span = super_class.span();
        if let Some(super_class) = self.transform_expression(super_class, false) {
          statements.push(self.ast.statement_expression(span, super_class));
        }
      }

      for element in &body.body {
        if let Some(key) = element.property_key()
          && key.is_expression()
          && let Some(element) = self.transform_expression(key.to_expression(), false)
        {
          statements.push(self.ast.statement_expression(element.span(), element));
        }
      }

      for element in &body.body {
        match element {
          ClassElement::StaticBlock(node) => {
            if let Some(node) = self.transform_static_block(node) {
              let StaticBlock { span, body, .. } = node.unbox();
              statements.push(self.ast.statement_block(span, body));
            }
          }
          ClassElement::PropertyDefinition(node) if node.r#static => {
            if let Some(value) = &node.value {
              let span = value.span();
              if let Some(value) = self.transform_expression(value, false) {
                statements.push(self.ast.statement_expression(span, value));
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
          self.ast.alloc_class(
            *span,
            *r#type,
            self.ast.vec(),
            (node.r#type == ClassType::ClassDeclaration)
              .then(|| self.build_unused_binding_identifier(id.as_ref().unwrap().span)),
            NONE,
            None,
            NONE,
            self.ast.vec(),
            self.ast.class_body(
              body.span(),
              self.ast.vec1(self.ast.class_element_static_block(body.span(), statements)),
            ),
            false,
            false,
          ),
        )
      }
    }
  }
}
