//! CPS transform for branching expressions: `ExprIf` and `ExprMatch`.
//!
//! Both forms **distribute** the continuation into every arm/branch:
//!
//! ```text
//! match scrutinee {        match scrutinee {
//!     Pat1 => body1,   →       Pat1 => { body1_lowered; cont },
//!     Pat2 => body2,           Pat2 => { body2_lowered; cont },
//! }                        }
//! k                        // (k is duplicated into each arm)
//! ```

mod if_;
mod match_;

pub(crate) use if_::{HoistTargetTest, default_hoist_test};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::cps::CpsStep;
    use crate::error::CpsError;
    use syn::{ExprIf, ExprMatch, parse_quote};

    #[test]
    fn if_with_attrs_rejected() {
        let if_e: ExprIf = parse_quote! {
            #[inline]
            if cond { 1 } else { 2 }
        };
        let err = if_e.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }

    #[test]
    fn match_with_attrs_rejected() {
        let match_e: ExprMatch = parse_quote! {
            #[cold]
            match x {
                _ => 1,
            }
        };
        let err = match_e.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}

#[cfg(test)]
mod let_chain_tests {
    use crate::cps::CpsContinuation;
    use crate::cps::CpsStep;
    use quote::quote;
    use syn::{ExprIf, parse_quote};

    #[test]
    fn let_chain_becomes_nested_if_conditions() {
        let if_expr: ExprIf = parse_quote! {
            if enabled && let Some(value) = candidate && value > 0 {
                value
            } else {
                0
            }
        };

        let normalized = if_expr.step().unwrap();
        let rendered = quote!(#normalized).to_string();

        assert!(!rendered.contains("&&"));
        assert_eq!(rendered.matches("if ").count(), 3);
        assert!(rendered.contains("let Some (value) = candidate"));
    }

    #[test]
    fn lowering_a_let_chain_keeps_the_pattern_available_to_the_body() {
        let if_expr: ExprIf = parse_quote! {
            if true && let Some(value) = candidate {
                consume(value);
            }
        };

        let lowered: CpsContinuation = if_expr.step().unwrap();
        let rendered = quote!(#lowered).to_string();

        assert!(rendered.contains("if true"));
        assert!(rendered.contains("if let Some (value)"));
        assert!(rendered.contains("consume (value)"));
    }
}
