use super::jsxs::create_react_jsxs_impl;
use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_jsx_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  // Because currently we don't track the detailed value of children, jsx and jsxs are the same.
  create_react_jsxs_impl(factory)
}
