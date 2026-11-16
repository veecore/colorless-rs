use crate::context::FoldContext;
use crate::util::{block, expr_path, ident_expr, reference, terminated_stmt};
use colorless_lower::util::syn::path_from_str;
use proc_macro2::{Ident, Span};
use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::{
    Expr, ExprCall, ExprClosure, Pat, Path, PathArguments, PathSegment, ReturnType, Stmt, Token,
    Type, parse_quote,
};

impl FoldContext {
    #[must_use]
    pub fn call(
        &self,
        capability: &str,
        method: &str,
        args: impl IntoIterator<Item = Expr>,
    ) -> Expr {
        make_call(self.call_path(capability, method), args)
    }

    #[must_use]
    pub fn call_path(&self, capability: &str, method: &str) -> Expr {
        self.call_path_with_color(&self.config.color_type, capability, method)
    }

    #[must_use]
    pub fn call_thunk(&self) -> Expr {
        expr_path(path_member(&self.config.crate_path, "call"))
    }

    #[must_use]
    pub fn call_infer_loop_step(&self, state: &Ident, step: Expr) -> Expr {
        let path = path_member(&self.config.crate_path, "infer_loop_step");
        make_call(expr_path(path), [reference(state.clone()), step])
    }

    #[must_use]
    pub fn call_assert_non_capturing(&self, state: &Ident, fn_: Ident) -> Stmt {
        let path = path_member(&self.config.crate_path, "assert_non_capturing");
        terminated_stmt(make_call(
            expr_path(path),
            [reference(state.clone()), ident_expr(fn_)],
        ))
    }

    pub fn call_into_colored(&self, expr: Expr) -> Expr {
        let krate = &self.config.crate_path;
        let color = &self.config.color_type;
        let path = parse_quote! {
            #krate::IntoColored::<#color>::into_colored
        };

        make_call(path, [expr])
    }

    #[must_use]
    pub fn identity_color(&self) -> Type {
        let krate = &self.config.crate_path;
        parse_quote! {
            #krate::Identity
        }
    }

    #[must_use]
    pub fn execute_identity(&self, expr: Expr) -> Expr {
        let krate = &self.config.crate_path;
        let path = parse_quote! {
            #krate::IdentityExt::into_value
        };

        make_call(path, [expr])
    }

    #[must_use]
    pub fn call_path_with_color(&self, color: &Type, capability: &str, method: &str) -> Expr {
        let capability = path_member(&self.config.crate_path, capability);
        let method = Ident::new(method, Span::call_site());

        parse_quote!(<#color as #capability>::#method)
    }

    #[must_use]
    pub fn cps_signal_crate(&self) -> &Path {
        &self.config.crate_path
    }

    #[must_use]
    pub fn break_loop(&self, val: Expr) -> Expr {
        let path = path_from_str("::core::ops::ControlFlow::Break");

        make_call(expr_path(path), [val])
    }

    #[must_use]
    pub fn continue_loop(&self, val: Expr) -> Expr {
        let path = path_from_str("::core::ops::ControlFlow::Continue");

        make_call(expr_path(path), [val])
    }

    #[must_use]
    pub fn shadow_path(&self) -> Path {
        path_member(&self.config.crate_path, "Shadowed")
    }

    #[must_use]
    pub fn call_store_type(&self, ident: &Ident, in_: &Ident) -> Stmt {
        let krate = &self.config.crate_path;
        parse_quote! {
            let #in_ = #krate::store_type(&#ident);
        }
    }

    #[must_use]
    pub fn wrap_in_maybe_unpinned(&self, l: &Ident) -> Expr {
        let krate = &self.config.crate_path;
        parse_quote! {
            #krate::MaybeUnpinned::new(#l)
        }
    }

    #[must_use]
    pub fn wrap_in_unsafe_fn_scope(&self, thunk: Expr) -> Expr {
        let krate = &self.config.crate_path;
        let path = parse_quote! {
            #krate::scope_fn
        };

        make_call(path, [thunk])
    }

    #[must_use]
    pub fn call_retrieve_type(&self, ident: &Ident, from: &Ident) -> Stmt {
        let krate = &self.config.crate_path;
        parse_quote! {
            let mut #ident = #krate::retrieve_type(#ident, #from);
        }
    }

    #[must_use]
    pub fn call_erased_as_mut(&self, ident: &Ident) -> Stmt {
        let krate = &self.config.crate_path;
        parse_quote! {
            let mut #ident = unsafe { #krate::ErasedBorrow::as_mut(&mut #ident) };
        }
    }

    #[must_use]
    pub fn call_unpin(&self, ident: &Ident) -> Expr {
        let krate = &self.config.crate_path;
        parse_quote! {
            unsafe { #krate::ErasedBorrow::unpin(#ident) }
        }
    }

    #[must_use]
    pub fn unify_value_path(&self) -> Path {
        let krate = &self.config.crate_path;
        parse_quote! {
            #krate::unify_value
        }
    }

    // fixme
    // #[must_use]
    // pub fn use_color(&self) -> Stmt {
    //     let krate = &self.config.crate_path;
    //     parse_quote! {
    //         #[allow(unused_imports)]
    //         use #krate::color;
    //     }
    // }
}

#[must_use]
pub(crate) fn path_member(base: &Path, member: &str) -> Path {
    let mut path = base.clone();
    path.segments.push(PathSegment {
        ident: format_ident!("{member}"),
        arguments: PathArguments::None,
    });
    path
}

#[must_use]
pub fn call_once_move(input: Pat, stmts: impl IntoIterator<Item = Stmt>) -> Expr {
    call_once(true, input, stmts)
}

#[must_use]
pub fn thunk(capture_env: bool, stmts: impl IntoIterator<Item = Stmt>) -> Expr {
    closure(capture_env, std::iter::empty(), block(stmts))
}

#[must_use]
pub fn call_once(capture_env: bool, input: Pat, stmts: impl IntoIterator<Item = Stmt>) -> Expr {
    closure(capture_env, std::iter::once(input), block(stmts))
}

#[must_use]
pub fn closure(capture_env: bool, inputs: impl IntoIterator<Item = Pat>, body: Expr) -> Expr {
    Expr::Closure(ExprClosure {
        attrs: Vec::new(),
        lifetimes: None,
        modifiers: Default::default(),
        constness: None,
        asyncness: None,
        capture: capture_env.then(Default::default),
        inputs_begin: Token![|](Span::call_site()),
        inputs: Punctuated::from_iter(inputs),
        inputs_end: Token![|](Span::call_site()),
        output: ReturnType::Default,
        body: Box::new(body),
    })
}

#[must_use]
pub fn make_call(func: Expr, args: impl IntoIterator<Item = Expr>) -> Expr {
    let call = ExprCall {
        attrs: vec![],
        func: Box::new(func),
        paren_token: Default::default(),
        args: Punctuated::from_iter(args),
    };

    Expr::Call(call)
}
