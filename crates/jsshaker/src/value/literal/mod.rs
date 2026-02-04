pub mod string;

use std::{fmt::Debug, vec};

use oxc::{
  allocator::Allocator,
  ast::ast::{BigintBase, Expression, NumberBase, UnaryOperator},
  semantic::SymbolId,
  span::{Atom, SPAN, Span},
};
use oxc_ecmascript::StringToNumber;
use oxc_syntax::number::ToJsString;

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, PropertyKeyValue, TypeofResult,
  ValueTrait, cacheable::Cacheable, consumed_object, never::NeverValue,
};
use crate::{
  analyzer::Analyzer,
  builtin_atom, builtin_string,
  builtins::BuiltinPrototype,
  dep::Dep,
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  transformer::Transformer,
  utils::F64WithEq,
  value::literal::string::ToAtomRef,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum LiteralValue<'a> {
  String(&'a Atom<'a>, Option<MangleAtom>),
  Number(F64WithEq),
  BigInt(&'a Atom<'a>),
  Boolean(bool),
  Symbol(SymbolId, &'a Atom<'a>),
  Null,
  Undefined,
}

impl<'a> ValueTrait<'a> for LiteralValue<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    if let LiteralValue::String(_, Some(atom)) = self {
      analyzer.consume(*atom);
    }
  }

  fn consume_mangable(&self, _analyzer: &mut Analyzer<'a>) -> bool {
    // No effect
    !matches!(self, LiteralValue::String(_, Some(_)))
  }

  fn unknown_mutate(&self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    match self {
      LiteralValue::Null | LiteralValue::Undefined => {
        analyzer.throw_builtin_error("Cannot get property of null or undefined");
        if analyzer.config.preserve_exceptions {
          consumed_object::get_property(self, analyzer, dep, key)
        } else {
          analyzer.factory.never
        }
      }
      LiteralValue::String(str, atom) => {
        let dep = analyzer.dep((dep, *atom));
        str.get_property(analyzer, dep, key)
      }
      _ => {
        let prototype = self.get_prototype(analyzer);
        prototype.get_property(analyzer, self.into(), key, dep)
      }
    }
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if matches!(self, LiteralValue::Null | LiteralValue::Undefined) {
      analyzer.throw_builtin_error("Cannot set property of null or undefined");
      if analyzer.config.preserve_exceptions {
        consumed_object::set_property(analyzer, dep, key, value)
      }
    } else {
      // No effect
    }
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if let LiteralValue::String(value, atom) = self {
      value.enumerate_properties(analyzer, analyzer.dep((dep, *atom)))
    } else {
      // No effect
      EnumeratedProperties { known: Default::default(), unknown: None, dep }
    }
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, _key: Entity<'a>) {
    if matches!(self, LiteralValue::Null | LiteralValue::Undefined) {
      analyzer.throw_builtin_error("Cannot delete property of null or undefined");
      if analyzer.config.preserve_exceptions {
        analyzer.consume(dep);
      }
    } else {
      // No effect
    }
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot call a non-function object {:?}", self));
    if analyzer.config.preserve_exceptions {
      consumed_object::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error(format!("Cannot construct a non-constructor object {:?}", self));
    if analyzer.config.preserve_exceptions {
      consumed_object::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, attributes: Entity<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown((self, attributes))
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    analyzer.factory.computed(self.into(), dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    match self {
      LiteralValue::String(value, atom) => (
        vec![],
        (!value.is_empty()).then_some(analyzer.factory.unknown_string),
        analyzer.dep((self, dep, *atom)),
      ),
      _ => {
        analyzer.throw_builtin_error("Cannot iterate over a non-iterable object");
        if analyzer.config.preserve_exceptions {
          self.consume(analyzer);
          consumed_object::iterate(analyzer, dep)
        } else {
          NeverValue.iterate(analyzer, dep)
        }
      }
    }
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer
      .factory
      .alloc(LiteralValue::String(
        self.to_string(analyzer.allocator),
        if let LiteralValue::String(_, Some(atom)) = self { Some(*atom) } else { None },
      ))
      .into()
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralValue::Number(_) | LiteralValue::BigInt(_) => self.into(),
      LiteralValue::Boolean(value) => {
        if *value {
          analyzer.factory.number(1.0)
        } else {
          analyzer.factory.number(0.0)
        }
      }
      LiteralValue::String(str, atom) => {
        analyzer.factory.computed(str.coerce_number(analyzer), *atom)
      }
      LiteralValue::Null => analyzer.factory.number(0.0),
      LiteralValue::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        analyzer.factory.unknown
      }
      LiteralValue::Undefined => analyzer.factory.nan,
    }
  }

  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(value) => analyzer.factory.boolean(value),
      None => analyzer.factory.computed_unknown_boolean(self),
    }
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralValue::Symbol(_, _) => self.into(),
      _ => self.coerce_string(analyzer),
    }
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if (TypeofResult::String | TypeofResult::Number).contains(self.test_typeof()) {
      self.coerce_string(analyzer)
    } else {
      builtin_string!("")
    }
  }

  fn get_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<PossibleLiterals<'a>> {
    Some(PossibleLiterals::Single(*self))
  }

  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    Some(*self)
  }

  fn get_own_keys(&self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    match self {
      LiteralValue::String(_, _) => None,
      _ => Some(vec![]),
    }
  }

  fn test_typeof(&self) -> TypeofResult {
    match self {
      LiteralValue::String(_, _) => TypeofResult::String,
      LiteralValue::Number(_) => TypeofResult::Number,
      LiteralValue::BigInt(_) => TypeofResult::BigInt,
      LiteralValue::Boolean(_) => TypeofResult::Boolean,
      LiteralValue::Symbol(_, _) => TypeofResult::Symbol,
      LiteralValue::Null => TypeofResult::Object,
      LiteralValue::Undefined => TypeofResult::Undefined,
    }
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(match self {
      LiteralValue::String(value, _) => !value.is_empty(),
      LiteralValue::Number(value) => {
        !value.0.is_nan() && *value != 0.0.into() && *value != (-0.0).into()
      }
      LiteralValue::BigInt(value) => !value.chars().all(|c| c == '0'),
      LiteralValue::Boolean(value) => *value,
      LiteralValue::Symbol(_, _) => true,
      LiteralValue::Null | LiteralValue::Undefined => false,
    })
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(matches!(self, LiteralValue::Null | LiteralValue::Undefined))
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    if let LiteralValue::String(s, _) = self {
      Some(Cacheable::String(s.as_str()))
    } else {
      Some(Cacheable::Literal(*self))
    }
  }
}

impl<'a> LiteralValue<'a> {
  pub fn build_expr(
    &self,
    transformer: &Transformer<'a>,
    span: Span,
    atom: Option<MangleAtom>,
  ) -> Expression<'a> {
    let ast = transformer.ast;
    match self {
      LiteralValue::String(value, _) => {
        let mut mangler = transformer.mangler.borrow_mut();
        let mangled = atom.and_then(|a| mangler.resolve(a)).unwrap_or(value);
        ast.expression_string_literal(span, mangled, None)
      }
      LiteralValue::Number(value) => {
        let negated = value.0.is_sign_negative();
        let absolute = if value.0.is_infinite() {
          ast.expression_identifier(span, "Infinity")
        } else if value.0.is_nan() {
          ast.expression_identifier(span, "NaN")
        } else {
          ast.expression_numeric_literal(span, value.0.abs(), None, NumberBase::Decimal)
        };
        if negated {
          ast.expression_unary(span, UnaryOperator::UnaryNegation, absolute)
        } else {
          absolute
        }
      }
      LiteralValue::BigInt(value) => {
        ast.expression_big_int_literal(span, **value, None, BigintBase::Decimal)
      }
      LiteralValue::Boolean(value) => ast.expression_boolean_literal(span, *value),
      LiteralValue::Symbol(_, _) => unreachable!("Cannot build expression for Symbol"),
      LiteralValue::Null => ast.expression_null_literal(span),
      LiteralValue::Undefined => ast.expression_unary(
        span,
        UnaryOperator::Void,
        ast.expression_numeric_literal(SPAN, 0.0, Some("0".into()), NumberBase::Decimal),
      ),
    }
  }

  pub fn can_build_expr(&self, analyzer: &Analyzer<'a>) -> bool {
    let config = &analyzer.config;
    match self {
      LiteralValue::String(value, _) => value.len() <= config.max_simple_string_length,
      LiteralValue::Number(value) => {
        !value.0.is_finite()
          || (value.0.fract() == 0.0
            && config.min_simple_number_value <= (value.0 as i64)
            && (value.0 as i64) <= config.max_simple_number_value)
      }
      LiteralValue::BigInt(_) => false,
      LiteralValue::Boolean(_) => true,
      LiteralValue::Symbol(_, _) => false,
      LiteralValue::Null => true,
      LiteralValue::Undefined => true,
    }
  }

  pub fn to_string(self, allocator: &'a Allocator) -> &'a Atom<'a> {
    match self {
      LiteralValue::String(value, _) => value,
      LiteralValue::Number(value) => value.0.to_js_string().to_atom_ref(allocator),
      LiteralValue::BigInt(value) => value,
      LiteralValue::Boolean(value) => {
        if value {
          builtin_atom!("true")
        } else {
          builtin_atom!("false")
        }
      }
      LiteralValue::Symbol(_, str_rep) => str_rep,
      LiteralValue::Null => builtin_atom!("null"),
      LiteralValue::Undefined => builtin_atom!("undefined"),
    }
  }

  // `None` for unresolvable, `Some(None)` for NaN, `Some(Some(value))` for number
  pub fn to_number(self) -> Option<Option<F64WithEq>> {
    match self {
      LiteralValue::Number(value) => Some(Some(value)),
      LiteralValue::BigInt(_value) => {
        // TODO: warn: TypeError: Cannot convert a BigInt value to a number
        None
      }
      LiteralValue::Boolean(value) => Some(Some(if value { 1.0 } else { 0.0 }.into())),
      LiteralValue::String(value, _) => {
        let val = value.trim().string_to_number();
        Some(if val.is_nan() { None } else { Some(val.into()) })
      }
      LiteralValue::Null => Some(Some(0.0.into())),
      LiteralValue::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        None
      }
      LiteralValue::Undefined => Some(None),
    }
  }

  fn get_prototype(&self, analyzer: &mut Analyzer<'a>) -> &'a BuiltinPrototype<'a> {
    match self {
      LiteralValue::String(_, _) => &analyzer.builtins.prototypes.string,
      LiteralValue::Number(_) => &analyzer.builtins.prototypes.number,
      LiteralValue::BigInt(_) => &analyzer.builtins.prototypes.bigint,
      LiteralValue::Boolean(_) => &analyzer.builtins.prototypes.boolean,
      LiteralValue::Symbol(_, _) => &analyzer.builtins.prototypes.symbol,
      LiteralValue::Null | LiteralValue::Undefined => {
        unreachable!("Cannot get prototype of null or undefined")
      }
    }
  }

  pub fn strict_eq(self, other: LiteralValue, object_is: bool) -> (bool, Option<MangleConstraint>) {
    if let (LiteralValue::Number(l), LiteralValue::Number(r)) = (self, other) {
      let eq = if object_is { l == r } else { l.0 == r.0 };
      return (eq, None);
    }

    if let (LiteralValue::String(l, atom_l), LiteralValue::String(r, atom_r)) = (self, other) {
      let eq = l == r;
      return (eq, MangleConstraint::equality(eq, atom_l, atom_r));
    }

    if self != other {
      return (false, None);
    }

    (true, None)
  }
}

impl<'a> From<LiteralValue<'a>> for PropertyKeyValue<'a> {
  fn from(val: LiteralValue<'a>) -> Self {
    match val {
      LiteralValue::String(s, _) => PropertyKeyValue::String(s),
      LiteralValue::Symbol(s, _) => PropertyKeyValue::Symbol(s),
      _ => unreachable!(),
    }
  }
}

impl<'a> From<LiteralValue<'a>> for (PropertyKeyValue<'a>, Option<MangleAtom>) {
  fn from(val: LiteralValue<'a>) -> Self {
    (
      val.into(),
      match val {
        LiteralValue::String(_, m) => m,
        _ => None,
      },
    )
  }
}

#[derive(Debug)]
pub enum PossibleLiterals<'a> {
  Single(LiteralValue<'a>),
  Multiple(Box<[LiteralValue<'a>]>),
}

impl<'a> PossibleLiterals<'a> {
  pub fn len(&self) -> usize {
    match self {
      PossibleLiterals::Single(_) => 1,
      PossibleLiterals::Multiple(lits) => lits.len(),
    }
  }
}

impl<'a, 'b> IntoIterator for &'b PossibleLiterals<'a> {
  type Item = &'b LiteralValue<'a>;
  type IntoIter = std::slice::Iter<'b, LiteralValue<'a>>;

  fn into_iter(self) -> Self::IntoIter {
    match self {
      PossibleLiterals::Single(lit) => std::slice::from_ref(lit).iter(),
      PossibleLiterals::Multiple(lits) => lits.iter(),
    }
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  pub fn number(&self, value: impl Into<F64WithEq>) -> Entity<'a> {
    self.alloc(LiteralValue::Number(value.into())).into()
  }

  pub fn big_int(&self, value: &'a Atom<'a>) -> Entity<'a> {
    self.alloc(LiteralValue::BigInt(value)).into()
  }

  pub fn boolean(&self, value: bool) -> Entity<'a> {
    if value { self.r#true } else { self.r#false }
  }

  pub fn boolean_maybe_unknown(&self, value: Option<bool>) -> Entity<'a> {
    if let Some(value) = value { self.boolean(value) } else { self.unknown_boolean }
  }

  pub fn symbol(&self, id: SymbolId, str_rep: &'a Atom<'a>) -> Entity<'a> {
    self.alloc(LiteralValue::Symbol(id, str_rep)).into()
  }
}
