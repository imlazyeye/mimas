#![allow(unused)]

use crate::components::{AdtId, Ty, Vid};
use colored::Colorize;
use parse::{
    expr::{Expr, Ident},
    stmt::Stmt,
};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    pub static PRINTER: RefCell<Printer> =
        RefCell::new(Printer {
        aliases: HashMap::default(),
        expr_strings: HashMap::default(),
        adt_names: HashMap::default(),
        alias_characters: vec![
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q',
            'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
            'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
            'z', 'Ƃ', 'Ɔ', 'ƈ', 'Ɖ', 'Ƌ', 'Ǝ', 'Ə', 'Ɛ', 'Ɣ', 'ƕ', 'Ɩ', 'Ɨ', 'ƚ', 'ƛ', 'Ɯ', 'ƞ',
            'Ɵ', 'ơ', 'Ƣ', 'Ƥ', 'ƥ', 'ƨ', 'Ʃ', 'ƫ', 'Ƭ', 'Ʊ', 'Ʋ', 'ƶ', 'ƹ', 'ƾ', 'ƿ',
        ],
        iter: 0,
    });
}

pub struct Printer {
    aliases: HashMap<Vid, String>,
    expr_strings: HashMap<Vid, String>,
    adt_names: HashMap<AdtId, String>,
    alias_characters: Vec<char>,
    iter: usize,
}
impl Printer {
    #[must_use]
    pub(crate) fn vid(vid: &Vid) -> String {
        PRINTER.with_borrow_mut(|printer| {
            if let Some(expr_string) = printer.expr_strings.get(vid) {
                expr_string.clone()
            } else {
                if let Some(entry) = printer.aliases.get(vid) {
                    entry.to_string()
                } else {
                    let v = {
                        let v = printer.alias_characters[printer.iter];
                        printer.iter = if printer.iter + 1 >= printer.alias_characters.len() {
                            0
                        } else {
                            printer.iter + 1
                        };
                        v.to_string()
                    };

                    printer.aliases.insert(*vid, v.clone());
                    v
                }
            }
        })
    }

    #[must_use]
    pub fn ty(ty: &Ty) -> String {
        match ty {
            Ty::Vid(vid) => Self::vid(vid),
            ty => ty.to_string(),
        }
    }

    #[must_use]
    pub(crate) fn stmt(a: &Stmt) -> String {
        format!("{}         {a}", "STMT".bright_white())
    }

    #[must_use]
    pub(crate) fn delete(a: &Vid) -> String {
        format!("{}       {}", "DELETE".bright_cyan(), Printer::vid(a))
    }

    #[must_use]
    pub(crate) fn query(a: &Expr) -> String {
        // todo, expr->vid lookup now lives on Solver, which the printer can't reach.
        format!("{}        {a}", "QUERY".bright_red())
        // format!(
        //     "{}        {a}: {}",
        //     "QUERY".bright_red(),
        //     Printer::Vid(&Vid::Expr(a.id())).bold().bright_black()
        // )
    }

    #[must_use]
    pub(crate) fn declare(ident: &Ident, ty: &Ty) -> String {
        format!(
            "{}      {ident}: {}",
            "DECLARE".bright_magenta(),
            Printer::ty(ty)
        )
    }

    #[must_use]
    pub(crate) fn new_declare(name: &str, ty: &Ty) -> String {
        format!(
            "{}      {name}: {}",
            "DECLARE".bright_magenta(),
            Printer::ty(ty)
        )
    }

    #[must_use]
    pub(crate) fn imp(adt: &str, name: &str, ty: &Ty) -> String {
        format!(
            "{}         {adt}::{name}: {}",
            "IMPL".bright_white(),
            Printer::ty(ty)
        )
    }

    #[must_use]
    pub(crate) fn ty_unification(a: &Ty, b: &Ty) -> String {
        format!(
            "{}        {}   ≟   {}",
            "UNIFY".bright_yellow(),
            Printer::ty(a).blue().bold(),
            Printer::ty(b).blue().bold(),
        )
    }

    #[must_use]
    pub(crate) fn substitution(vid: &Vid, ty: &Ty) -> String {
        format!(
            "{}          {}   ->   {}",
            "SUB".bright_green(),
            Printer::vid(vid).bright_black().bold(),
            Printer::ty(ty).blue().bold(),
        )
    }

    pub(crate) fn report_new_adt(id: AdtId, name: String) {
        PRINTER.with_borrow_mut(|p| p.adt_names.insert(id, name));
    }

    pub(crate) fn adt_name(id: AdtId) -> Option<String> {
        PRINTER.with_borrow(|p| p.adt_names.get(&id).cloned())
    }
}
