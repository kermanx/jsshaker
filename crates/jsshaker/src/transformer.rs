use std::{
  cell::{Cell, RefCell},
  hash::{DefaultHasher, Hasher},
  rc::Rc,
};

use oxc::{
  allocator::{Allocator, CloneIn},
  ast::{
    AstBuilder, NONE,
    ast::{
      AssignmentTarget, BinaryOperator, BindingIdentifier, BindingPattern, BindingPatternKind,
      Expression, ForStatementLeft, FormalParameterKind, IdentifierReference, LogicalOperator,
      NumberBase, Program, SimpleAssignmentTarget, Statement, UnaryOperator,
      VariableDeclarationKind,
    },
  },
  semantic::{ScopeId, Semantic, SymbolId},
  span::{Atom, GetSpan, SPAN, Span},
};
use rustc_hash::FxHashMap;

use crate::{
  TreeShakeConfig, analyzer::conditional::ConditionalDataMap, dep::DeoptimizedAtoms,
  folding::ConstantFolder, mangling::Mangler, utils::ExtraData,
};

pub struct Transformer<'a> {
  pub config: &'a TreeShakeConfig,
  pub allocator: &'a Allocator,
  pub path: Atom<'a>,
  pub data: &'a ExtraData<'a>,
  pub deoptimized_atoms: &'a DeoptimizedAtoms,
  pub conditional_data: &'a ConditionalDataMap<'a>,
  pub folder: &'a ConstantFolder<'a>,
  pub mangler: Rc<RefCell<&'a mut Mangler<'a>>>,
  pub semantic: Semantic<'a>,

  pub ast: AstBuilder<'a>,

  pub var_decls: RefCell<FxHashMap<SymbolId, bool>>,
  /// The block statement has already exited, so we can and only can transform declarations themselves
  pub declaration_only: Cell<bool>,
  pub need_unused_assignment_target: Cell<bool>,
  pub need_non_nullish_helper: Cell<bool>,
  pub unused_identifier_names: RefCell<FxHashMap<u64, usize>>,
}

impl<'a> Transformer<'a> {
  pub fn new(
    config: &'a TreeShakeConfig,
    allocator: &'a Allocator,
    path: Atom<'a>,
    data: &'a ExtraData<'a>,
    deoptimized_atoms: &'a DeoptimizedAtoms,
    conditional_data: &'a ConditionalDataMap<'a>,
    folder: &'a ConstantFolder<'a>,
    mangler: Rc<RefCell<&'a mut Mangler<'a>>>,
    semantic: Semantic<'a>,
  ) -> Self {
    Transformer {
      config,
      allocator,
      path,
      data,
      deoptimized_atoms,
      conditional_data,
      folder,
      mangler,
      semantic,

      ast: AstBuilder::new(allocator),

      var_decls: Default::default(),
      declaration_only: Cell::new(false),
      need_unused_assignment_target: Cell::new(false),
      need_non_nullish_helper: Cell::new(false),
      unused_identifier_names: Default::default(),
    }
  }

  pub fn transform_program(&self, node: &'a Program<'a>) -> Program<'a> {
    let Program { span, source_type, source_text, comments, hashbang, directives, body, .. } = node;

    let mut transformed_body = self.ast.vec();

    for statement in body {
      if let Some(statement) = self.transform_statement(statement) {
        transformed_body.push(statement);
      }
    }

    self.patch_var_declarations(node.scope_id.get().unwrap(), &mut transformed_body);

    if self.need_unused_assignment_target.get() {
      transformed_body.push(self.build_unused_assignment_target_definition());
    }
    if self.need_non_nullish_helper.get() {
      transformed_body.push(self.build_non_nullish_helper_definition());
    }

    self.ast.program(
      *span,
      *source_type,
      source_text,
      self.clone_node(comments),
      self.clone_node(hashbang),
      self.clone_node(directives),
      transformed_body,
    )
  }

  pub fn update_var_decl_state(&self, symbol: SymbolId, is_declaration: bool) {
    if !self.semantic.scoping().symbol_flags(symbol).is_function_scoped_declaration() {
      return;
    }
    let mut var_decls = self.var_decls.borrow_mut();
    if is_declaration {
      var_decls.insert(symbol, false);
    } else {
      var_decls.entry(symbol).or_insert(true);
    }
  }

  /// Append missing var declarations at the end of the function body or program
  pub fn patch_var_declarations(
    &self,
    scope_id: ScopeId,
    statements: &mut oxc::allocator::Vec<'a, Statement<'a>>,
  ) {
    let bindings = self.semantic.scoping().get_bindings(scope_id);
    if bindings.is_empty() {
      return;
    }

    let var_decls = self.var_decls.borrow();
    let mut declarations = self.ast.vec();
    for symbol_id in bindings.values() {
      if var_decls.get(symbol_id) == Some(&true) {
        let name = self.semantic.scoping().symbol_name(*symbol_id);
        let span = self.semantic.scoping().symbol_span(*symbol_id);
        declarations.push(self.ast.variable_declarator(
          span,
          VariableDeclarationKind::Var,
          self.ast.binding_pattern(
            self.ast.binding_pattern_kind_binding_identifier(span, self.ast.atom(name)),
            NONE,
            false,
          ),
          None,
          false,
        ));
      }
    }

    if !declarations.is_empty() {
      statements.push(Statement::from(self.ast.declaration_variable(
        SPAN,
        VariableDeclarationKind::Var,
        declarations,
        false,
      )));
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn clone_node<T: CloneIn<'a>>(&self, node: &T) -> T::Cloned {
    node.clone_in(self.allocator)
  }

  pub fn build_unused_binding_identifier(&self, span: Span) -> BindingIdentifier<'a> {
    let text = self.semantic.source_text().as_bytes();
    let start = 5.max(span.start as usize) - 5;
    let end = text.len().min(span.end as usize + 5);

    let mut hasher = DefaultHasher::new();
    hasher.write(&text[start..end]);
    let hash = hasher.finish() % 0xFFFF;
    let index =
      *self.unused_identifier_names.borrow_mut().entry(hash).and_modify(|e| *e += 1).or_insert(0);
    let name = if index == 0 {
      format!("__unused_{:04X}", hash)
    } else {
      format!("__unused_{:04X}_{}", hash, index - 1)
    };
    self.ast.binding_identifier(span, self.ast.atom(&name))
  }

  pub fn build_unused_binding_pattern(&self, span: Span) -> BindingPattern<'a> {
    self.ast.binding_pattern(
      BindingPatternKind::BindingIdentifier(
        self.ast.alloc(self.build_unused_binding_identifier(span)),
      ),
      NONE,
      false,
    )
  }

  pub fn build_unused_assignment_binding_pattern(&self, span: Span) -> BindingPattern<'a> {
    self.ast.binding_pattern(
      self.ast.binding_pattern_kind_assignment_pattern(
        span,
        self.build_unused_binding_pattern(SPAN),
        self.build_unused_expression(SPAN),
      ),
      NONE,
      false,
    )
  }

  pub fn build_unused_identifier_reference_write(&self, span: Span) -> IdentifierReference<'a> {
    self.need_unused_assignment_target.set(true);
    self.ast.identifier_reference(span, "__unused__")
  }

  pub fn build_unused_simple_assignment_target(&self, span: Span) -> SimpleAssignmentTarget<'a> {
    SimpleAssignmentTarget::AssignmentTargetIdentifier(
      self.ast.alloc(self.build_unused_identifier_reference_write(span)),
    )
  }

  pub fn build_unused_assignment_target(&self, span: Span) -> AssignmentTarget<'a> {
    // The commented doesn't work because nullish value can't be destructured
    // self.ast.assignment_target_assignment_target_pattern(
    //   self.ast.assignment_target_pattern_object_assignment_target(
    //     span,
    //     self.ast.vec(),
    //     None,
    //   ),
    // )
    AssignmentTarget::from(self.build_unused_simple_assignment_target(span))
  }

  pub fn build_unused_assignment_target_in_rest(&self, span: Span) -> AssignmentTarget<'a> {
    AssignmentTarget::from(self.build_unused_simple_assignment_target(span))
  }

  pub fn build_unused_for_statement_left(&self, span: Span) -> ForStatementLeft<'a> {
    ForStatementLeft::from(self.build_unused_assignment_target(span))
  }

  pub fn build_unused_expression(&self, span: Span) -> Expression<'a> {
    self.ast.expression_numeric_literal(span, 0.0f64, None, NumberBase::Decimal)
  }

  pub fn build_undefined(&self, span: Span) -> Expression<'a> {
    self.ast.expression_identifier(span, "undefined")
  }

  pub fn build_negate_expression(&self, expression: Expression<'a>) -> Expression<'a> {
    self.ast.expression_unary(expression.span(), UnaryOperator::LogicalNot, expression)
  }

  pub fn build_object_spread_effect(&self, span: Span, argument: Expression<'a>) -> Expression<'a> {
    self.ast.expression_object(
      span,
      self.ast.vec1(self.ast.object_property_kind_spread_property(span, argument)),
    )
  }

  pub fn build_unused_assignment_target_definition(&self) -> Statement<'a> {
    Statement::from(self.ast.declaration_variable(
      SPAN,
      VariableDeclarationKind::Var,
      self.ast.vec1(self.ast.variable_declarator(
        SPAN,
        VariableDeclarationKind::Var,
        self.ast.binding_pattern(
          self.ast.binding_pattern_kind_binding_identifier(SPAN, "__unused__"),
          NONE,
          false,
        ),
        None,
        false,
      )),
      false,
    ))
  }

  pub fn build_non_nullish_helper_definition(&self) -> Statement<'a> {
    Statement::from(self.ast.declaration_variable(
      SPAN,
      VariableDeclarationKind::Var,
      self.ast.vec1(self.ast.variable_declarator(
        SPAN,
        VariableDeclarationKind::Var,
        self.ast.binding_pattern(
          self.ast.binding_pattern_kind_binding_identifier(SPAN, "__non_nullish__"),
          NONE,
          false,
        ),
        Some(self.ast.expression_arrow_function(
          SPAN,
          true,
          false,
          NONE,
          self.ast.formal_parameters(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            self.ast.vec1(self.ast.formal_parameter(
              SPAN,
              self.ast.vec(),
              self.ast.binding_pattern(
                self.ast.binding_pattern_kind_binding_identifier(SPAN, "v"),
                NONE,
                false,
              ),
              None,
              false,
              false,
            )),
            NONE,
          ),
          NONE,
          self.ast.function_body(
            SPAN,
            self.ast.vec(),
            self.ast.vec1(self.ast.statement_expression(
              SPAN,
              self.ast.expression_logical(
                SPAN,
                self.ast.expression_binary(
                  SPAN,
                  self.ast.expression_identifier(SPAN, "v"),
                  BinaryOperator::StrictInequality,
                  self.ast.expression_null_literal(SPAN),
                ),
                LogicalOperator::And,
                self.ast.expression_binary(
                  SPAN,
                  self.ast.expression_identifier(SPAN, "v"),
                  BinaryOperator::StrictInequality,
                  self.ast.expression_identifier(SPAN, "undefined"),
                ),
              ),
            )),
          ),
        )),
        false,
      )),
      false,
    ))
  }

  pub fn build_chain_expression_mock(
    &self,
    span: Span,
    left: Expression<'a>,
    right: Expression<'a>,
  ) -> Expression<'a> {
    self.need_non_nullish_helper.set(true);
    self.ast.expression_logical(
      span,
      self.ast.expression_call(
        left.span(),
        self.ast.expression_identifier(span, "__non_nullish__"),
        NONE,
        self.ast.vec1(left.into()),
        false,
      ),
      LogicalOperator::And,
      right,
    )
  }
}
