use crate::util::thunk_block;
use syn::punctuated::Punctuated;
use syn::visit_mut::{VisitMut, visit_block_mut};
use syn::{
    Arm, Block, Expr, ExprBlock, ExprCall, ExprMatch, ExprPath, Member, Path, Stmt, Type,
    parse_quote,
};

pub struct Inert {
    pub block: Block,
    // this is to prevent further occurrence of the bug
    pub resolve_encountered: bool,
}

/// Constructs an inert copy of the input so that rustc enforces
/// native borrow checking, lifetime validation, and let-else divergence checks before CPS lowering.
pub fn generate_inert_body(mut block: Block, color_type: &Type, crate_path: &Path) -> Inert {
    struct ReplaceResolve<'a> {
        color_type: &'a Type,
        crate_path: &'a Path,
        iter_counter: usize,
        resolve_encountered: bool,
    }

    // fixme: do not go into opaques
    // #[colorless]
    // fn ex16() {
    //     let f = || { ready(1).resolve };
    //     f();
    // }
    // this panics at runtime because codegen found no resolve, used what we generated here
    // believing it's plain, but we already rewrote ready(1) to panic
    impl VisitMut for ReplaceResolve<'_> {
        fn visit_expr_mut(&mut self, expr: &mut Expr) {
            let attrs = match expr {
                Expr::ForLoop(loop_) => Some(&mut loop_.attrs),
                Expr::Loop(loop_) => Some(&mut loop_.attrs),
                Expr::While(loop_) => Some(&mut loop_.attrs),
                _ => None,
            };
            const LOOP_STATE: &str = "state";
            const NO_SHADOW: &str = "no_shadow";
            const DEFAULT_BREAK_STATE: &str = "default_state";
            if let Some(attrs) = attrs {
                attrs.retain(|attr| {
                    attr.path().segments.last().is_none_or(|segment| {
                        !matches!(
                            segment.ident.to_string().as_str(),
                            LOOP_STATE | NO_SHADOW | DEFAULT_BREAK_STATE
                        )
                    })
                });
            }

            if let Expr::ForLoop(for_loop) = expr {
                let mut is_colorless = false;
                for_loop.attrs.retain(|attr| {
                    if attr
                        .path()
                        .segments
                        .last()
                        .is_some_and(|seg| seg.ident == "colorless")
                    {
                        is_colorless = true;
                        false
                    } else {
                        true
                    }
                });

                if is_colorless {
                    self.visit_expr_mut(&mut for_loop.expr);
                    self.visit_block_mut(&mut for_loop.body);

                    let iter_ident = quote::format_ident!("__iter_{}", self.iter_counter);
                    self.iter_counter += 1;

                    let crate_path = self.crate_path;
                    let color_type = self.color_type;
                    let for_expr = &for_loop.expr;
                    let pat = &for_loop.pat;
                    let body = &for_loop.body;
                    let label = &for_loop.label;
                    let attrs = &for_loop.attrs;

                    *expr = parse_quote! {
                        match #crate_path::iter::IntoColoredIterator::<#color_type>::into_iter(#for_expr) {
                            mut #iter_ident => #(#attrs)* #label loop {
                                match #crate_path::__compiler::resolve_unreachable::<#color_type, _>(
                                    #iter_ident.next()
                                ).resolve {
                                    ::core::option::Option::None => break,
                                    ::core::option::Option::Some(#pat) => #body,
                                }
                            }
                        }
                    };
                    return;
                }
            }

            syn::visit_mut::visit_expr_mut(self, expr);

            let Expr::Field(field) = expr else {
                return;
            };
            if !matches!(&field.member, Member::Named(member) if member == "resolve") {
                return;
            }

            self.resolve_encountered = true;

            let crate_path = self.crate_path;
            let color_type = self.color_type;

            let base = &field.base;

            field.base = Expr::Call(ExprCall {
                attrs: vec![],
                func: Box::new(Expr::Path(ExprPath {
                    attrs: vec![],
                    qself: None,
                    path: parse_quote! {
                        #crate_path::__compiler::resolve_unreachable<#color_type, _>
                    },
                })),
                paren_token: Default::default(),
                args: Punctuated::from_iter([*base.clone()]),
            })
            .into();
        }
    }

    let mut replace_resolve = ReplaceResolve {
        color_type,
        crate_path,
        iter_counter: 0,
        resolve_encountered: false,
    };

    visit_block_mut(&mut replace_resolve, &mut block);
    let mut stmts = Vec::with_capacity(block.stmts.len() + 1);
    // If no resolve, remove all imports as they can panic if used... stop this once we
    // stop going into opaques
    if replace_resolve.resolve_encountered {
        stmts.push(parse_quote! {
            #[allow(unused_imports)]
            use #crate_path::__compiler::{
                __borrow as borrow, __rb as rb, __unborrow as unborrow, __pin as pin, __unpin as unpin, keep_alive,
                __slot as slot, __take_slot as take_slot, __rbv as rbv
            };
        });
    }

    stmts.append(&mut block.stmts);
    let block = Block { stmts, ..block };
    Inert {
        block,
        resolve_encountered: replace_resolve.resolve_encountered,
    }
}

pub fn with_original_validation(
    crate_path: &Path,
    lowered: Expr,
    inert: Block,
    capture_env: bool,
) -> Expr {
    let hint = Expr::Call(ExprCall {
        attrs: vec![],
        func: parse_quote!(#crate_path::__compiler::never_execute_inert),
        paren_token: Default::default(),
        args: Punctuated::from_iter([thunk_block(inert, capture_env)]),
    });

    fn block(e: Expr) -> Box<Expr> {
        Box::new(Expr::Block(ExprBlock {
            attrs: vec![],
            label: None,
            block: Block {
                brace_token: Default::default(),
                stmts: vec![Stmt::Expr(e, None)],
            },
        }))
    }

    let inert_arm = Arm {
        attrs: vec![],
        pat: parse_quote!(false),
        fat_arrow_token: Default::default(),
        body: block(hint),
        comma: None,
    };

    // unused_variables because of macro capture... still don't get it
    let colorless_arm = Arm {
        attrs: parse_quote! {
            #[allow(path_statements, unused, clippy::unused_unit, nonstandard_style)]
        },
        pat: parse_quote!(true),
        fat_arrow_token: Default::default(),
        body: block(lowered),
        comma: None,
    };
    // we no longer use if because it doesn't allow disabling lints
    Expr::Match(ExprMatch {
        attrs: vec![],
        match_token: Default::default(),
        expr: parse_quote!(true),
        brace_token: Default::default(),
        arms: vec![inert_arm, colorless_arm],
    })
}

#[cfg(any())]
#[test]
fn lint_disabling() {
    match true {
        #[allow(unused_mut)]
        false => {
            let mut x = 1;
            println!("{x}")
        }
        true => {
            let mut x = 1;
            println!("{x}")
        }
    }
}
