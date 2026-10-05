//! # `colorless-cps`
//!
//! Compiler pass transforming `syn` ASTs into Continuation-Passing Style (CPS)
//! and A-Normal Form (ANF) with jump-free and label-free intermediate representation.

pub mod ir {
    //! Intermediate Representation (IR) and conversion DSL.
}

pub mod cps {
    //! Continuation-Passing Style lowering pipeline.
}
