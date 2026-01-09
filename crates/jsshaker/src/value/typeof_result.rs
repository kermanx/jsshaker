use bitflags::bitflags;

use crate::{analyzer::Factory, builtin_string, entity::Entity};

bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct TypeofResult: u8 {
    const _None = 0;
    const _Unknown = 0xFF;
    const _Primitive = Self::String.bits()
      | Self::Number.bits()
      | Self::BigInt.bits()
      | Self::Boolean.bits()
      | Self::Symbol.bits()
      | Self::Undefined.bits();

    const String = 1 << 0;
    const Number = 1 << 1;
    const BigInt = 1 << 2;
    const Boolean = 1 << 3;
    const Symbol = 1 << 4;
    const Undefined = 1 << 5;
    const Object = 1 << 6;
    const Function = 1 << 7;
  }
}

impl TypeofResult {
  pub fn to_entity<'a>(self, factory: &Factory<'a>) -> Entity<'a> {
    match self.bits().count_ones() {
      0 => factory.never,
      n if n >= 8 => factory.unknown_string,
      _ => {
        let mut values = factory.vec();
        if self.contains(TypeofResult::String) {
          values.push(builtin_string!("string"));
        }
        if self.contains(TypeofResult::Number) {
          values.push(builtin_string!("number"));
        }
        if self.contains(TypeofResult::BigInt) {
          values.push(builtin_string!("bigint"));
        }
        if self.contains(TypeofResult::Boolean) {
          values.push(builtin_string!("boolean"));
        }
        if self.contains(TypeofResult::Symbol) {
          values.push(builtin_string!("symbol"));
        }
        if self.contains(TypeofResult::Undefined) {
          values.push(builtin_string!("undefined"));
        }
        if self.contains(TypeofResult::Object) {
          values.push(builtin_string!("object"));
        }
        if self.contains(TypeofResult::Function) {
          values.push(builtin_string!("function"));
        }
        factory.union(values)
      }
    }
  }

  pub fn must_equal(self, other: TypeofResult) -> bool {
    self == other && self.bits().count_ones() == 1
  }
}
