//! CPS lowering for non-local jumps (`ExprBreak`, `ExprContinue`, `ExprReturn`, and `?` `ExprTry`).

pub mod break_;
pub mod continue_;
pub mod return_;
pub mod try_;

// TODO: IF THE CONTINUATION IS CONTINUE OR RETURN WE SHOULD BE ABLE TO UNSET THE
//  LOGGED CONTINUES SELF AND RETURNS WHEN APPENDED TO AN UNTERMINATING EXPR
