use std::sync::atomic::{AtomicUsize, Ordering};

use proc_macro2::Span;
use quote::format_ident;
use syn::parse_quote;

use crate::FieldIdent;

#[derive(Default)]
pub struct Counter(AtomicUsize);

impl Counter {
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    pub fn get(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct SymbolGenerator {
    devtree: syn::Path,
    lt_blob: syn::Lifetime,
}

impl SymbolGenerator {
    pub fn new(devtree: syn::Path, blob: syn::Lifetime) -> Self {
        Self {
            devtree,
            lt_blob: blob,
        }
    }

    pub fn private(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::__private }
    }

    pub fn lt_blob(&self) -> &syn::Lifetime {
        &self.lt_blob
    }
}

pub fn gen_field_var(field_ident: &FieldIdent) -> syn::Ident {
    match field_ident {
        FieldIdent::Named(field_ident) => {
            format_ident!("__devtree_f_{field_ident}", span = Span::call_site())
        }
        FieldIdent::Unnamed(index) => {
            format_ident!("__devtree_f_{index}", span = Span::call_site())
        }
    }
}

pub fn gen_generic_param_d() -> syn::GenericParam {
    parse_quote! { __DEVTREE_D }
}

pub fn gen_var(name: &str) -> syn::Ident {
    static ID: Counter = Counter::new();
    format_ident!("__devtree_v_{name}_{}", ID.get())
}

pub fn gen_lt(name: &str) -> syn::Lifetime {
    static ID: Counter = Counter::new();
    let n = ID.get();
    syn::Lifetime::new(&format!("'__devtree_{name}_{n}"), Span::call_site())
}
