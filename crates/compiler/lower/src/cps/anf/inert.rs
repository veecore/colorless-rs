use crate::ir::{CpsExpr, CpsExprArray, CpsExprTuple, CpsInert};
use crate::{derive_hoist_target, derive_hoist_target_inert};
use syn::{ExprArray, ExprAsync, ExprClosure, ExprConst, ExprInfer, ExprLit, ExprPath, ExprTuple};

derive_hoist_target! {
    ExprArray {
        _ attrs,
        bracket_token,
        (pub elems),
    } -> {
        Array(CpsExprArray)
    }
}

derive_hoist_target_inert! {
    Async(ExprAsync),
    Closure(ExprClosure),
    Const(ExprConst),
    Infer(ExprInfer),
    Lit(ExprLit),
    Path(ExprPath)
}

derive_hoist_target! {
    ExprTuple {
        _ attrs,
        paren_token,
        (pub elems),
    } -> {
        Tuple(CpsExprTuple)
    }
}
