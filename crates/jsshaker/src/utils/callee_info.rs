use std::{hash, ptr};

use oxc::{
  ast::{
    AstKind,
    ast::{ArrowFunctionExpression, Class, Function, PropertyKind},
  },
  semantic::ScopeId,
  span::{GetSpan, Span},
};
use oxc_index::define_nonmax_u32_index_type;

use super::ast::AstKind2;
use crate::{analyzer::Analyzer, module::ModuleId, value::bound::BoundFunction};

#[derive(Debug, Clone, Copy)]
pub enum CalleeNode<'a> {
  Function(&'a Function<'a>),
  ArrowFunctionExpression(&'a ArrowFunctionExpression<'a>),
  ClassStatics(&'a Class<'a>),
  ClassConstructor(&'a Class<'a>),
  BoundFunction(&'a BoundFunction<'a>),
  Root,
  Module,
}

impl<'a> From<CalleeNode<'a>> for AstKind2<'a> {
  fn from(val: CalleeNode<'a>) -> Self {
    match val {
      CalleeNode::Function(node) => AstKind2::Function(node),
      CalleeNode::ArrowFunctionExpression(node) => AstKind2::ArrowFunctionExpression(node),
      CalleeNode::ClassStatics(node) => AstKind2::Class(node),
      CalleeNode::ClassConstructor(node) => AstKind2::ClassConstructor(node),
      CalleeNode::BoundFunction(_) | CalleeNode::Root | CalleeNode::Module => AstKind2::Environment,
    }
  }
}

impl GetSpan for CalleeNode<'_> {
  fn span(&self) -> Span {
    match self {
      CalleeNode::Function(node) => node.span(),
      CalleeNode::ArrowFunctionExpression(node) => node.span(),
      CalleeNode::ClassStatics(node) => node.span(),
      CalleeNode::ClassConstructor(node) => node.span(),
      CalleeNode::BoundFunction(f) => f.span,
      CalleeNode::Root | CalleeNode::Module => Span::default(),
    }
  }
}

impl PartialEq for CalleeNode<'_> {
  fn eq(&self, other: &Self) -> bool {
    match (*self, *other) {
      (CalleeNode::Module, CalleeNode::Module) => true,
      (CalleeNode::Function(a), CalleeNode::Function(b)) => ptr::eq(a, b),
      (CalleeNode::ArrowFunctionExpression(a), CalleeNode::ArrowFunctionExpression(b)) => {
        ptr::eq(a, b)
      }
      (CalleeNode::ClassStatics(a), CalleeNode::ClassStatics(b)) => ptr::eq(a, b),
      (CalleeNode::ClassConstructor(a), CalleeNode::ClassConstructor(b)) => ptr::eq(a, b),
      (CalleeNode::BoundFunction(a), CalleeNode::BoundFunction(b)) => a.span == b.span,
      _ => false,
    }
  }
}

impl Eq for CalleeNode<'_> {}

impl hash::Hash for CalleeNode<'_> {
  fn hash<H: hash::Hasher>(&self, state: &mut H) {
    self.span().hash(state)
  }
}

define_nonmax_u32_index_type! {
  pub struct CalleeInstanceId;
}

#[derive(Debug, Clone, Copy)]
pub struct CalleeInfo<'a> {
  pub module_id: ModuleId,
  pub node: CalleeNode<'a>,
  pub instance_id: CalleeInstanceId,
  #[cfg(feature = "flame")]
  pub debug_name: &'a str,
}

impl<'a> CalleeInfo<'a> {
  pub fn into_node(self) -> AstKind2<'a> {
    self.node.into()
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_callee_info(&self, node: CalleeNode<'a>) -> CalleeInfo<'a> {
    CalleeInfo {
      module_id: self.current_module(),
      node,
      instance_id: self.factory.alloc_instance_id(),
      #[cfg(feature = "flame")]
      debug_name: {
        let file_name = self.module_info().path.as_str();
        let line_col = self.line_index().line_col(node.span().start.into());
        let resolved_name = match node {
          CalleeNode::Function(node) => {
            if let Some(id) = &node.id {
              &id.name
            } else {
              self.resolve_function_name(node.scope_id()).unwrap_or("<unnamed>")
            }
          }
          CalleeNode::ArrowFunctionExpression(node) => {
            self.resolve_function_name(node.scope_id()).unwrap_or("<anonymous>")
          }
          CalleeNode::ClassStatics(_) => "<ClassStatics>",
          CalleeNode::ClassConstructor(_) => "<ClassConstructor>",
          CalleeNode::BoundFunction(_) => "<BoundFunction>",
          CalleeNode::Root => "<Root>",
          CalleeNode::Module => "<Module>",
        };
        let debug_name =
          format!("{}[./{}:{}:{}]", resolved_name, file_name, line_col.line + 1, line_col.col + 1);
        self.allocator.alloc_str(&debug_name)
      },
    }
  }

  /// Note: this is for flamegraph only. May not conform to the standard.
  #[allow(dead_code)]
  fn resolve_function_name(&self, scope_id: ScopeId) -> Option<&'a str> {
    let node_id = self.semantic().scoping().get_node_id(scope_id);
    let parent = self.semantic().nodes().parent_kind(node_id);
    match parent {
      AstKind::VariableDeclarator(node) => node.id.get_identifier_name().map(|a| a.as_str()),
      AstKind::AssignmentPattern(node) => node.left.get_identifier_name().map(|a| a.as_str()),
      AstKind::AssignmentExpression(node) => node.left.get_identifier_name(),
      AstKind::ObjectProperty(node) => node.key.static_name().map(|s| {
        let kind_text = match node.kind {
          PropertyKind::Init => "",
          PropertyKind::Get => "get ",
          PropertyKind::Set => "set ",
        };
        self.allocator.alloc_str(&(kind_text.to_string() + &s))
      }),
      AstKind::ImportSpecifier(node) => Some(node.imported.name().as_str()),
      _ => None,
    }
  }
}
