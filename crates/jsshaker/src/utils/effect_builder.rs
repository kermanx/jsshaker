use oxc::{allocator, ast::ast::Expression};

pub trait AppendEffect<'a> {
  fn append_effect(self, effects: &mut allocator::Vec<Expression<'a>>);
}

impl<'a> AppendEffect<'a> for Expression<'a> {
  fn append_effect(self, effects: &mut allocator::Vec<Expression<'a>>) {
    effects.push(self);
  }
}

impl<'a, T: AppendEffect<'a>> AppendEffect<'a> for Vec<T> {
  fn append_effect(self, effects: &mut allocator::Vec<Expression<'a>>) {
    for item in self.into_iter() {
      item.append_effect(effects);
    }
  }
}

impl<'a, T: AppendEffect<'a>> AppendEffect<'a> for Option<T> {
  fn append_effect(self, effects: &mut allocator::Vec<Expression<'a>>) {
    if let Some(item) = self {
      item.append_effect(effects);
    }
  }
}

#[macro_export]
macro_rules! build_effect {
  ($builder:expr, $span:expr, $($x:expr),+ $(,)?) => {
    {
      use $crate::utils::effect_builder::AppendEffect;
      let mut effects = $builder.vec();
      $($x.append_effect(&mut effects);)*
      if effects.is_empty() {
        None
      }
      else if effects.len() == 1 {
        effects.pop()
      }
      else {
        Some($builder.expression_sequence($span, effects))
      }
    }
  };
  ($builder:expr, $span:expr, $($x:expr),+ $(,)?; $val:expr) => {
    {
      use $crate::utils::effect_builder::AppendEffect;
      let mut effects = $builder.vec();
      $($x.append_effect(&mut effects);)*
      if effects.is_empty() {
        $val
      }
      else {
        effects.push($val);
        $builder.expression_sequence($span, effects)
      }
    }
  };
}
