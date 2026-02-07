use std::fmt::{self, Debug};

use oxc::{
  allocator::Vec,
  ast::ast::*,
  span::{GetSpan, SPAN},
};

use crate::{analyzer::Analyzer, dep::CustomDepTrait};

pub type Arguments<'a> = Vec<'a, Argument<'a>>;

macro_rules! ast_kind_2 {
  ($($x:ident($t:ty)),+ $(,)?) => {
    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    #[repr(u8)]
    pub enum AstKind2<'a> {
      Index(usize),
      $( $x($t), )+
    }

    impl<'a> AstKind2<'a> {
      pub const ENVIRONMENT: Self = AstKind2::Index(0);
    }

    impl GetSpan for AstKind2<'_> {
      fn span(&self) -> Span {
        match self {
          AstKind2::Index(_) => SPAN,
          $( AstKind2::$x(node) => node.span(), )+
        }
      }
    }

    impl fmt::Debug for AstKind2<'_> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
          AstKind2::Index(index) => write!(f, "Index({})", index),
          $(AstKind2::$x(node) => {
            // Example: "IdentifierName(111-222)"
            let span = node.span();
            let name = stringify!($x);
            write!(f, "{}({}-{})", name, span.start, span.end)
          })+
        }
      }
    }
  };
}

ast_kind_2! {
  NumericLiteral(&'a NumericLiteral<'a>),
  BigIntLiteral(&'a BigIntLiteral<'a>),
  RegExpLiteral(&'a RegExpLiteral<'a>),
  StringLiteral(&'a StringLiteral<'a>),
  Program(&'a Program<'a>),
  IdentifierName(&'a IdentifierName<'a>),
  IdentifierReference(&'a IdentifierReference<'a>),
  BindingIdentifier(&'a BindingIdentifier<'a>),
  LabelIdentifier(&'a LabelIdentifier<'a>),
  ThisExpression(&'a ThisExpression),
  ArrayExpression(&'a ArrayExpression<'a>),
  ArrayExpressionElement(&'a ArrayExpressionElement<'a>),
  Elision(&'a Elision),
  ObjectExpression(&'a ObjectExpression<'a>),
  ObjectProperty(&'a ObjectProperty<'a>),
  PropertyKey(&'a PropertyKey<'a>),
  TemplateLiteral(&'a TemplateLiteral<'a>),
  TaggedTemplateExpression(&'a TaggedTemplateExpression<'a>),
  MemberExpression(&'a MemberExpression<'a>),
  CallExpression(&'a CallExpression<'a>),
  NewExpression(&'a NewExpression<'a>),
  MetaProperty(&'a MetaProperty<'a>),
  SpreadElement(&'a SpreadElement<'a>),
  Argument(&'a Argument<'a>),
  UpdateExpression(&'a UpdateExpression<'a>),
  UnaryExpression(&'a UnaryExpression<'a>),
  BinaryExpression(&'a BinaryExpression<'a>),
  PrivateInExpression(&'a PrivateInExpression<'a>),
  LogicalExpression(&'a LogicalExpression<'a>),
  ConditionalExpression(&'a ConditionalExpression<'a>),
  AssignmentExpression(&'a AssignmentExpression<'a>),
  AssignmentTarget(&'a AssignmentTarget<'a>),
  SimpleAssignmentTarget(&'a SimpleAssignmentTarget<'a>),
  AssignmentTargetPattern(&'a AssignmentTargetPattern<'a>),
  ArrayAssignmentTarget(&'a ArrayAssignmentTarget<'a>),
  ObjectAssignmentTarget(&'a ObjectAssignmentTarget<'a>),
  AssignmentTargetWithDefault(&'a AssignmentTargetWithDefault<'a>),
  SequenceExpression(&'a SequenceExpression<'a>),
  Super(&'a Super),
  AwaitExpression(&'a AwaitExpression<'a>),
  ChainExpression(&'a ChainExpression<'a>),
  ParenthesizedExpression(&'a ParenthesizedExpression<'a>),
  Directive(&'a Directive<'a>),
  Hashbang(&'a Hashbang<'a>),
  BlockStatement(&'a BlockStatement<'a>),
  VariableDeclaration(&'a VariableDeclaration<'a>),
  VariableDeclarator(&'a VariableDeclarator<'a>),
  EmptyStatement(&'a EmptyStatement),
  ExpressionStatement(&'a ExpressionStatement<'a>),
  IfStatement(&'a IfStatement<'a>),
  DoWhileStatement(&'a DoWhileStatement<'a>),
  WhileStatement(&'a WhileStatement<'a>),
  ForStatement(&'a ForStatement<'a>),
  ForStatementInit(&'a ForStatementInit<'a>),
  ForInStatement(&'a ForInStatement<'a>),
  ForOfStatement(&'a ForOfStatement<'a>),
  ContinueStatement(&'a ContinueStatement<'a>),
  BreakStatement(&'a BreakStatement<'a>),
  ReturnStatement(&'a ReturnStatement<'a>),
  WithStatement(&'a WithStatement<'a>),
  SwitchStatement(&'a SwitchStatement<'a>),
  SwitchCase(&'a SwitchCase<'a>),
  LabeledStatement(&'a LabeledStatement<'a>),
  ThrowStatement(&'a ThrowStatement<'a>),
  TryStatement(&'a TryStatement<'a>),
  FinallyClause(&'a BlockStatement<'a>),
  CatchClause(&'a CatchClause<'a>),
  CatchParameter(&'a CatchParameter<'a>),
  DebuggerStatement(&'a DebuggerStatement),
  AssignmentPattern(&'a AssignmentPattern<'a>),
  ObjectPattern(&'a ObjectPattern<'a>),
  ArrayPattern(&'a ArrayPattern<'a>),
  BindingRestElement(&'a BindingRestElement<'a>),
  Function(&'a Function<'a>),
  FormalParameters(&'a FormalParameters<'a>),
  FormalParameter(&'a FormalParameter<'a>),
  FunctionBody(&'a FunctionBody<'a>),
  ArrowFunctionExpression(&'a ArrowFunctionExpression<'a>),
  YieldExpression(&'a YieldExpression<'a>),
  Class(&'a Class<'a>),
  ClassHeritage(&'a Expression<'a>),
  ClassBody(&'a ClassBody<'a>),
  MethodDefinition(&'a MethodDefinition<'a>),
  PropertyDefinition(&'a PropertyDefinition<'a>),
  PrivateIdentifier(&'a PrivateIdentifier<'a>),
  StaticBlock(&'a StaticBlock<'a>),
  Decorator(&'a Decorator<'a>),
  ModuleDeclaration(&'a ModuleDeclaration<'a>),
  ImportExpression(&'a ImportExpression<'a>),
  ImportDeclaration(&'a ImportDeclaration<'a>),
  ImportSpecifier(&'a ImportSpecifier<'a>),
  ImportDefaultSpecifier(&'a ImportDefaultSpecifier<'a>),
  ImportNamespaceSpecifier(&'a ImportNamespaceSpecifier<'a>),
  ExportNamedDeclaration(&'a ExportNamedDeclaration<'a>),
  ExportDefaultDeclaration(&'a ExportDefaultDeclaration<'a>),
  ExportAllDeclaration(&'a ExportAllDeclaration<'a>),
  ExportSpecifier(&'a ExportSpecifier<'a>),
  JSXAttributeItem(&'a JSXAttributeItem<'a>),
  JSXMemberExpression(&'a JSXMemberExpression<'a>),
  JsxExpressionContainer(&'a JSXExpressionContainer<'a>),

  // extras
  Expression(&'a Expression<'a>),
  ObjectPropertyKey(&'a ObjectProperty<'a>),
  SwitchCaseTest(&'a SwitchCase<'a>),
  AssignmentTargetProperty(&'a AssignmentTargetProperty<'a>),
  AssignmentTargetPropertyIdentifier(&'a AssignmentTargetPropertyIdentifier<'a>),
  AssignmentTargetRest(&'a AssignmentTargetRest<'a>),
  BindingProperty(&'a BindingProperty<'a>),
  Callee(&'a Expression<'a>),
  ClassConstructor(&'a Class<'a>),
  ExpressionInTaggedTemplate(&'a Expression<'a>),
  LogicalExpressionLeft(&'a LogicalExpression<'a>),
  LogicalAssignmentExpressionLeft(&'a AssignmentExpression<'a>),
  JSXOpeningElement(&'a JSXOpeningElement<'a>),
  JSXAttributeName(&'a JSXAttributeName<'a>),
  ArrowFunctionBodyExecuted(&'a FunctionBody<'a>),
  WithDefault(&'a Expression<'a>),
  FunctionNoShake(&'a Function<'a>),
  JSXIdentifier(&'a JSXIdentifier<'a>),
}

impl<'a> CustomDepTrait<'a> for AstKind2<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.include_atom(*self);
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclarationKind {
  Var,
  UntrackedVar,
  Let,
  Const,
  Function,
  NamedFunctionInBody,
  Class,
  Import,
  Caught,
  FunctionParameter,
  ArrowFunctionParameter,
}

impl DeclarationKind {
  pub fn is_var(self) -> bool {
    matches!(self, DeclarationKind::Var | DeclarationKind::UntrackedVar)
  }

  pub fn is_untracked(self) -> bool {
    matches!(self, DeclarationKind::UntrackedVar)
  }

  pub fn is_const(self) -> bool {
    matches!(self, DeclarationKind::Const | DeclarationKind::NamedFunctionInBody)
  }

  pub fn is_redeclarable(self) -> bool {
    matches!(
      self,
      DeclarationKind::Var
        | DeclarationKind::UntrackedVar
        | DeclarationKind::Function
        | DeclarationKind::Class
    )
  }

  pub fn is_shadowable(self) -> bool {
    self.is_redeclarable()
      || matches!(
        self,
        DeclarationKind::FunctionParameter
          | DeclarationKind::ArrowFunctionParameter
          | DeclarationKind::Caught
      )
  }
}
