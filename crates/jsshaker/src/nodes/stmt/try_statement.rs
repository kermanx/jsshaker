use oxc::ast::ast::{Statement, TryStatement};

use crate::{analyzer::Analyzer, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_try_statement(&mut self, node: &'a TryStatement<'a>) {
    self.push_non_det_cf_scope();
    if self.scoping.try_catch_depth.is_none() {
      self.scoping.try_catch_depth = Some(self.scoping.cf.current_depth());
      self.exec_block_statement(&node.block);
      self.scoping.try_catch_depth = None;
    } else {
      self.exec_block_statement(&node.block);
    }
    let mut try_scope = self.pop_cf_scope();
    // Preserving the throw's control flow only matters when dead code can
    // actually be shaken out; otherwise this is pure overhead.
    let exit_dep = if self.config.advanced { try_scope.deps.collect(self.factory) } else { None };

    if let Some(handler) = &node.handler {
      self.exec_catch_clause(handler, self.factory.unknown, exit_dep);
    }

    if let Some(finalizer) = &node.finalizer {
      self.exec_block_statement(finalizer);
    }

    if self.config.advanced && node.handler.is_none() && try_scope.thrown {
      // An exception thrown in the block may propagate through the finalizer to
      // the outer scopes. This is always a conditional exit: `break`/`return`
      // traversals have already marked the outer scopes, and re-marking them
      // must-exited would be wrong since the try block itself may not run to
      // the throw.
      let target_depth = self.scoping.try_catch_depth.unwrap_or(0);
      self.exit_to_impl(target_depth, self.scoping.cf.stack_len(), false, exit_dep);
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_try_statement(&self, node: &'a TryStatement<'a>) -> Option<Statement<'a>> {
    let TryStatement { span, block, handler, finalizer } = node;

    let block = self.transform_block_statement(block);

    let handler_span = handler.as_ref().map(|handler| handler.span).unwrap_or_default();
    let handler = if block.is_some() {
      handler.as_ref().map(|handler| self.transform_catch_clause(handler))
    } else {
      None
    };

    let finalizer =
      finalizer.as_ref().and_then(|finalizer| self.transform_block_statement(finalizer));

    match (block, finalizer) {
      (None, None) => None,
      (None, Some(finalizer)) => Some(Statement::BlockStatement(finalizer)),
      (Some(block), finalizer) => Some(self.ast.statement_try(
        *span,
        block,
        if finalizer.is_some() {
          handler
        } else {
          Some(handler.unwrap_or_else(|| {
            self.ast.catch_clause(
              handler_span,
              None,
              self.ast.block_statement(handler_span, self.ast.vec()),
            )
          }))
        },
        finalizer,
      )),
    }
  }
}
