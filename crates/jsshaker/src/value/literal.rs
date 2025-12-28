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
  builtins::BuiltinPrototype,
  dep::Dep,
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  transformer::Transformer,
  utils::F64WithEq,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum LiteralValue<'a> {
  String(&'a str, Option<MangleAtom>),
  Number(F64WithEq, Option<&'a str>),
  BigInt(&'a str),
  Boolean(bool),
  Symbol(SymbolId, &'a str),
  Infinity(bool),
  NaN,
  Null,
  Undefined,
}

impl<'a> ValueTrait<'a> for LiteralValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if let LiteralValue::String(_, Some(atom)) = self {
      analyzer.consume(*atom);
    }
  }

  fn consume_mangable(&'a self, _analyzer: &mut Analyzer<'a>) -> bool {
    // No effect
    !matches!(self, LiteralValue::String(_, Some(_)))
  }

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if matches!(self, LiteralValue::Null | LiteralValue::Undefined) {
      analyzer.throw_builtin_error("Cannot get property of null or undefined");
      if analyzer.config.preserve_exceptions {
        consumed_object::get_property(self, analyzer, dep, key)
      } else {
        analyzer.factory.never
      }
    } else {
      let prototype = self.get_prototype(analyzer);
      let dep = analyzer.dep((self, dep, key));
      if let Some(key_literals) = key.get_to_literals(analyzer) {
        let mut values = analyzer.factory.vec();
        for key_literal in key_literals {
          if let Some(property) = self.get_known_instance_property(analyzer, key_literal) {
            values.push(property);
          } else if let Some(property) = prototype.get_literal_keyed(key_literal) {
            values.push(property);
          } else {
            values.push(analyzer.factory.unmatched_prototype_property);
          }
        }
        analyzer.factory.computed_union(values, dep)
      } else {
        analyzer.factory.computed_unknown(dep)
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
      let dep = analyzer.dep((dep, *atom));
      if value.len() <= analyzer.config.max_simple_string_length {
        EnumeratedProperties {
          known: value
            .char_indices()
            .map(|(i, c)| {
              let i_str = analyzer.allocator.alloc_str(&i.to_string());
              (
                PropertyKeyValue::String(i_str),
                (
                  true,
                  analyzer.factory.unmangable_string(i_str),
                  analyzer.factory.unmangable_string(analyzer.allocator.alloc_str(&c.to_string())),
                ),
              )
            })
            .collect(),
          unknown: None,
          dep,
        }
      } else {
        analyzer.factory.computed_unknown_string(self).enumerate_properties(analyzer, dep)
      }
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

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer
      .factory
      .alloc(LiteralValue::String(
        self.to_string(analyzer.allocator),
        if let LiteralValue::String(_, Some(atom)) = self { Some(*atom) } else { None },
      ))
      .into()
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralValue::Number(_, _)
      | LiteralValue::BigInt(_)
      | LiteralValue::NaN
      | LiteralValue::Infinity(_) => self.into(),
      LiteralValue::Boolean(value) => {
        if *value {
          analyzer.factory.number(1.0, Some("1"))
        } else {
          analyzer.factory.number(0.0, Some("0"))
        }
      }
      LiteralValue::String(str, atom) => {
        let val = str.string_to_number();
        analyzer.factory.computed(
          if val.is_nan() { analyzer.factory.nan } else { analyzer.factory.number(val, None) },
          *atom,
        )
      }
      LiteralValue::Null => analyzer.factory.number(0.0, Some("0")),
      LiteralValue::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        analyzer.factory.unknown
      }
      LiteralValue::Undefined => analyzer.factory.nan,
    }
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(value) => analyzer.factory.boolean(value),
      None => analyzer.factory.computed_unknown_boolean(self),
    }
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      LiteralValue::Symbol(_, _) => self.into(),
      _ => self.get_to_string(analyzer),
    }
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if (TypeofResult::String | TypeofResult::Number).contains(self.test_typeof()) {
      self.get_to_string(analyzer)
    } else {
      analyzer.factory.builtin_string("")
    }
  }

  fn get_to_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<LiteralValue<'a>>> {
    Some(vec![*self])
  }

  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    Some(*self)
  }

  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    match self {
      LiteralValue::String(_, _) => None,
      _ => Some(vec![]),
    }
  }

  fn test_typeof(&self) -> TypeofResult {
    match self {
      LiteralValue::String(_, _) => TypeofResult::String,
      LiteralValue::Number(_, _) => TypeofResult::Number,
      LiteralValue::BigInt(_) => TypeofResult::BigInt,
      LiteralValue::Boolean(_) => TypeofResult::Boolean,
      LiteralValue::Symbol(_, _) => TypeofResult::Symbol,
      LiteralValue::Infinity(_) => TypeofResult::Number,
      LiteralValue::NaN => TypeofResult::Number,
      LiteralValue::Null => TypeofResult::Object,
      LiteralValue::Undefined => TypeofResult::Undefined,
    }
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(match self {
      LiteralValue::String(value, _) => !value.is_empty(),
      LiteralValue::Number(value, _) => *value != 0.0.into() && *value != (-0.0).into(),
      LiteralValue::BigInt(value) => !value.chars().all(|c| c == '0'),
      LiteralValue::Boolean(value) => *value,
      LiteralValue::Symbol(_, _) => true,
      LiteralValue::Infinity(_) => true,
      LiteralValue::NaN | LiteralValue::Null | LiteralValue::Undefined => false,
    })
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(matches!(self, LiteralValue::Null | LiteralValue::Undefined))
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    if let LiteralValue::String(s, _) = self {
      Some(Cacheable::Literal(LiteralValue::String(s, None)))
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
      LiteralValue::Number(value, raw) => {
        let negated = value.0.is_sign_negative();
        let absolute = ast.expression_numeric_literal(
          span,
          value.0.abs(),
          raw.map(Atom::from),
          NumberBase::Decimal,
        );
        if negated {
          ast.expression_unary(span, UnaryOperator::UnaryNegation, absolute)
        } else {
          absolute
        }
      }
      LiteralValue::BigInt(value) => {
        ast.expression_big_int_literal(span, *value, None, BigintBase::Decimal)
      }
      LiteralValue::Boolean(value) => ast.expression_boolean_literal(span, *value),
      LiteralValue::Symbol(_, _) => unreachable!("Cannot build expression for Symbol"),
      LiteralValue::Infinity(positive) => {
        if *positive {
          ast.expression_identifier(span, "Infinity")
        } else {
          ast.expression_unary(
            span,
            UnaryOperator::UnaryNegation,
            ast.expression_identifier(span, "Infinity"),
          )
        }
      }
      LiteralValue::NaN => ast.expression_identifier(span, "NaN"),
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
      LiteralValue::Number(value, _) => {
        value.0.fract() == 0.0
          && config.min_simple_number_value <= (value.0 as i64)
          && (value.0 as i64) <= config.max_simple_number_value
      }
      LiteralValue::BigInt(_) => false,
      LiteralValue::Boolean(_) => true,
      LiteralValue::Symbol(_, _) => false,
      LiteralValue::Infinity(_) => true,
      LiteralValue::NaN => true,
      LiteralValue::Null => true,
      LiteralValue::Undefined => true,
    }
  }

  pub fn to_string(self, allocator: &'a Allocator) -> &'a str {
    match self {
      LiteralValue::String(value, _) => value,
      LiteralValue::Number(value, str_rep) => {
        str_rep.unwrap_or_else(|| allocator.alloc_str(&value.0.to_js_string()))
      }
      LiteralValue::BigInt(value) => value,
      LiteralValue::Boolean(value) => {
        if value {
          "true"
        } else {
          "false"
        }
      }
      LiteralValue::Symbol(_, str_rep) => str_rep,
      LiteralValue::Infinity(positive) => {
        if positive {
          "Infinity"
        } else {
          "-Infinity"
        }
      }
      LiteralValue::NaN => "NaN",
      LiteralValue::Null => "null",
      LiteralValue::Undefined => "undefined",
    }
  }

  // `None` for unresolvable, `Some(None)` for NaN, `Some(Some(value))` for number
  pub fn to_number(self) -> Option<Option<F64WithEq>> {
    match self {
      LiteralValue::Number(value, _) => Some(Some(value)),
      LiteralValue::BigInt(_value) => {
        // TODO: warn: TypeError: Cannot convert a BigInt value to a number
        None
      }
      LiteralValue::Boolean(value) => Some(Some(if value { 1.0 } else { 0.0 }.into())),
      LiteralValue::String(value, _) => {
        let value = value.trim();
        Some(if value.is_empty() {
          Some(0.0.into())
        } else if let Ok(value) = value.parse::<f64>() {
          Some(value.into())
        } else {
          None
        })
      }
      LiteralValue::Null => Some(Some(0.0.into())),
      LiteralValue::Symbol(_, _) => {
        // TODO: warn: TypeError: Cannot convert a Symbol value to a number
        None
      }
      LiteralValue::NaN | LiteralValue::Undefined => Some(None),
      LiteralValue::Infinity(_) => None,
    }
  }

  fn get_prototype(&self, analyzer: &mut Analyzer<'a>) -> &'a BuiltinPrototype<'a> {
    match self {
      LiteralValue::String(_, _) => &analyzer.builtins.prototypes.string,
      LiteralValue::Number(_, _) => &analyzer.builtins.prototypes.number,
      LiteralValue::BigInt(_) => &analyzer.builtins.prototypes.bigint,
      LiteralValue::Boolean(_) => &analyzer.builtins.prototypes.boolean,
      LiteralValue::Symbol(_, _) => &analyzer.builtins.prototypes.symbol,
      LiteralValue::Infinity(_) => &analyzer.builtins.prototypes.number,
      LiteralValue::NaN => &analyzer.builtins.prototypes.number,
      LiteralValue::Null | LiteralValue::Undefined => {
        unreachable!("Cannot get prototype of null or undefined")
      }
    }
  }

  fn get_known_instance_property(
    &self,
    analyzer: &Analyzer<'a>,
    key: LiteralValue<'a>,
  ) -> Option<Entity<'a>> {
    match self {
      LiteralValue::String(value, atom_self) => {
        let LiteralValue::String(key, atom_key) = key else { return None };
        if key == "length" {
          Some(analyzer.factory.number(value.len() as f64, None))
        } else if let Ok(index) = key.parse::<usize>() {
          Some(
            value
              .get(index..index + 1)
              .map_or(analyzer.factory.undefined, |v| analyzer.factory.unmangable_string(v)),
          )
        } else {
          None
        }
        .map(|val| analyzer.factory.computed(val, (*atom_self, atom_key)))
      }
      _ => None,
    }
  }

  pub fn strict_eq(self, other: LiteralValue, object_is: bool) -> (bool, Option<MangleConstraint>) {
    // 0.0 === -0.0
    if !object_is && let (LiteralValue::Number(l, _), LiteralValue::Number(r, _)) = (self, other) {
      let eq = if l == 0.0.into() || l == (-0.0).into() {
        r == 0.0.into() || r == (-0.0).into()
      } else {
        l == r
      };
      return (eq, None);
    }

    if let (LiteralValue::String(l, atom_l), LiteralValue::String(r, atom_r)) = (self, other) {
      let eq = l == r;
      return (eq, MangleConstraint::equality(eq, atom_l, atom_r));
    }

    if self != other {
      return (false, None);
    }

    if !object_is && self == LiteralValue::NaN {
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
