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

    pub fn lt_blob(&self) -> &syn::Lifetime {
        &self.lt_blob
    }

    pub fn trait_deserialize_property(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::DeserializeProperty }
    }

    pub fn trait_deserialize_node(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::DeserializeNode }
    }

    pub fn trait_property_collection(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::PropertyCollection }
    }

    pub fn trait_node_collection(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::NodeCollection }
    }

    pub fn trait_deserialize_node_with_lifetime(&self, blob: &syn::Lifetime) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::DeserializeNode::<#blob> }
    }

    pub fn trait_property_deserializer(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::PropertyDeserializer }
    }

    pub fn trait_node_deserializer(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::NodeDeserializer }
    }

    pub fn trait_node_deserializer_with_lt(
        &self,
        lt_de: &syn::Lifetime,
        lt_blob: &syn::Lifetime,
    ) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::NodeDeserializer::<#lt_de, #lt_blob> }
    }

    pub fn ty_error(&self) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::error::DeserializeError }
    }

    pub fn ty_item_deserializer(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::ItemDeserializer }
    }

    pub fn ty_property(&self) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::blob::Property }
    }

    pub fn ty_node(&self) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::blob::Node }
    }

    pub fn ty_prop_cell_with_ty(&self, ty: &syn::Type) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::util::PropertyCell::<#ty> }
    }

    pub fn ty_node_cell_with_ty(&self, ty: &syn::Type) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::util::NodeCell::<#ty> }
    }

    pub fn expr_property_deserializer(&self, ty: &syn::Type) -> syn::Expr {
        let trait_ = self.trait_deserialize_property();
        parse_quote! { <#ty as #trait_>::deserialize_property }
    }

    pub fn expr_node_deserializer(&self, ty: &syn::Type) -> syn::Expr {
        let trait_ = self.trait_deserialize_node();
        parse_quote! { <#ty as #trait_>::deserialize_node }
    }

    pub fn expr_property_collection_inserter(&self, ty: &syn::Type) -> syn::Expr {
        let trait_ = self.trait_property_collection();
        parse_quote! { <#ty as #trait_>::insert_property }
    }

    pub fn expr_node_collection_inserter(&self, ty: &syn::Type) -> syn::Expr {
        let trait_ = self.trait_node_collection();
        parse_quote! { <#ty as #trait_>::insert_node }
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

pub fn path_core() -> syn::Path {
    parse_quote! { ::core }
}

pub fn expr_default(ty: &syn::Type) -> syn::Expr {
    let core = path_core();
    parse_quote! {
        <#ty as #core::default::Default>::default()
    }
}

pub fn expr_ok(expr: &syn::Expr) -> syn::Expr {
    let result = ty_result();
    parse_quote! {
        #result::Ok(#expr)
    }
}

pub fn ty_result() -> syn::Path {
    let core = path_core();
    parse_quote! {
        #core::result::Result
    }
}

pub fn ty_result_with_param(ok: &syn::Type, err: &syn::Type) -> syn::Type {
    let core = path_core();
    parse_quote! {
        #core::result::Result::<#ok, #err>
    }
}

pub fn trait_sized() -> syn::Type {
    let core = path_core();
    parse_quote! {
        #core::marker::Sized
    }
}
