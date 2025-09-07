use proc_macro2::Span;
use quote::format_ident;
use syn::parse_quote;

use crate::FieldIdent;

#[derive(Debug)]
pub struct SymbolGenerator {
    devtree: syn::Path,
    blob: syn::Lifetime,
}

impl SymbolGenerator {
    pub fn new(devtree: syn::Path, blob: syn::Lifetime) -> Self {
        Self { devtree, blob }
    }

    pub fn blob(&self) -> &syn::Lifetime {
        &self.blob
    }

    pub fn trait_deserialize_property(&self) -> syn::Path {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::DeserializeProperty }
    }

    pub fn trait_deserialize_node_with_lifetime(&self) -> syn::Path {
        let devtree = &self.devtree;
        let blob = &self.blob;
        parse_quote! { #devtree::de::DeserializeNode<#blob> }
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

    pub fn ty_node_context(&self) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::NodeContext }
    }

    pub fn ty_node_context_with_lifetime(&self) -> syn::Type {
        let devtree = &self.devtree;
        let blob = &self.blob;
        parse_quote! { #devtree::de::NodeContext<'_, #blob> }
    }

    pub fn ty_error(&self) -> syn::Type {
        let devtree = &self.devtree;
        parse_quote! { #devtree::de::DeserializeError }
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

pub fn gen_ctx_var(target: &str, kind: &str) -> syn::Ident {
    let k = match kind {
        "node" => "n",
        "prop" => "p",
        _ => panic!("unknown context kind: {kind}"),
    };
    if target == "self" {
        format_ident!("__devtree_c_{k}ctx", span = Span::call_site())
    } else {
        assert!(target == "parent" || target == "child");
        format_ident!("__devtree_c_{target}_{k}ctx", span = Span::call_site())
    }
}

pub fn gen_local_var(name: &str) -> syn::Ident {
    format_ident!("__devtree_v_{name}")
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

pub fn ty_option() -> syn::Path {
    let core = path_core();
    parse_quote! {
        #core::option::Option
    }
}

pub fn expr_none(ty: &syn::Type) -> syn::Expr {
    let option = ty_option();
    parse_quote! {
        #option::<#ty>::None
    }
}

pub fn expr_some(expr: &syn::Expr) -> syn::Expr {
    let option = ty_option();
    parse_quote! {
        #option::Some(#expr)
    }
}

pub fn ty_result() -> syn::Path {
    let core = path_core();
    parse_quote! {
        #core::result::Result
    }
}
