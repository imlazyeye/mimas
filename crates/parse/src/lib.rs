// #![warn(missing_docs)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::print_stdout)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::similar_names)]
#![warn(clippy::todo)]
#![warn(clippy::unimplemented)]
#![warn(clippy::undocumented_unsafe_blocks)]
// temporary
#![allow(clippy::result_large_err)]

pub mod lex {
    mod lexer;
    mod tok;
    pub use lexer::*;
    pub use tok::*;
}

pub mod components {
    mod annotation;
    mod binding;
    mod macros;
    mod pat;
    pub use annotation::*;
    pub use binding::*;
    pub use pat::*;
}

pub mod expr {
    #[allow(clippy::module_inception)]
    mod expr;
    pub use expr::*;
    mod expr_kinds {
        mod absolve;
        mod access;
        mod block;
        mod r#break;
        mod call;
        mod closure;
        mod coalescence;
        mod collect;
        mod r#continue;
        mod equality;
        mod evaluation;
        mod r#for;
        mod fstring;
        mod grouping;
        mod ident;
        mod r#if;
        mod r#in;
        mod literal;
        mod logical;
        mod r#loop;
        mod r#match;
        mod raise;
        mod range;
        mod r#return;
        mod unary;
        mod unwrap;
        mod r#while;
        pub use absolve::*;
        pub use access::*;
        pub use block::*;
        pub use r#break::*;
        pub use call::*;
        pub use closure::*;
        pub use coalescence::*;
        pub use collect::*;
        pub use r#continue::*;
        pub use equality::*;
        pub use evaluation::*;
        pub use r#for::*;
        pub use fstring::*;
        pub use grouping::*;
        pub use ident::*;
        pub use r#if::*;
        pub use r#in::*;
        pub use literal::*;
        pub use logical::*;
        pub use r#loop::*;
        pub use r#match::*;
        pub use raise::*;
        pub use range::*;
        pub use r#return::*;
        pub use unary::*;
        pub use unwrap::*;
        pub use r#while::*;
    }
    pub use expr_kinds::*;
}

pub mod stmt {
    #[allow(clippy::module_inception)]
    mod stmt;
    pub use stmt::*;
    mod stmt_kinds {
        mod assignment;
        mod r#let;
        mod module;
        pub use assignment::*;
        pub use r#let::*;
        pub use module::*;
    }
    pub use stmt_kinds::*;
}

pub mod item {
    #[allow(clippy::module_inception)]
    mod item;
    pub use item::*;
    mod item_kinds {
        mod r#const;
        mod r#enum;
        mod function;
        mod r#impl;
        mod pact;
        mod r#struct;
        mod r#use;
        pub use r#const::*;
        pub use r#enum::*;
        pub use function::*;
        pub use r#impl::*;
        pub use pact::*;
        pub use r#struct::*;
        pub use r#use::*;
    }
    pub use item_kinds::*;
}

mod ast;
pub mod errors;
mod parser;
pub use ast::*;
pub use expr::*;
pub use item::*;
pub use parser::*;
pub use stmt::*;

#[cfg(test)]
mod tests {
    #[macro_use]
    mod utils;

    mod autogen_tok_tests;
    mod lex_tokens;
    mod parse_exprs;
    mod parse_stmts;
    mod parse_validity;
}
