use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

use crate::{
    ChildSpec, Fallback, PropertySpec, RepeatedChildrenSpec, ResolvedName, SymbolGenerator,
    meta::{ExtraChildrenSpec, ExtraPropertiesSpec, NodeSpec},
    sgen,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldIdent {
    Named(syn::Ident),
    Unnamed(usize),
}

impl quote::ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(ident) => ident.to_tokens(tokens),
            Self::Unnamed(index) => {
                let index = syn::Index::from(*index);
                index.to_tokens(tokens);
            }
        }
    }
}

#[derive(Debug)]
pub struct Builder {
    sgen: SymbolGenerator,
    ident: syn::Ident,
    generics: syn::Generics,
    node_ctx: syn::Ident,
    prop_ctx: syn::Ident,
    child_node_ctx: syn::Ident,
    var_defs: Vec<syn::Stmt>,
    property_handlers: Vec<(ResolvedName, syn::Expr)>,
    extra_properties_handler: Option<syn::Expr>,
    node_handlers: Vec<(ResolvedName, syn::Expr)>,
    field_values: Vec<(FieldIdent, syn::Expr)>,
    extra_nodes_handler: Option<syn::Expr>,
}

impl Builder {
    pub fn new(
        devtree: syn::Path,
        blob: syn::Lifetime,
        ident: syn::Ident,
        generics: syn::Generics,
    ) -> Self {
        let sgen = SymbolGenerator::new(devtree, blob);
        let nctx = sgen::gen_ctx_var("self", "node");
        let pctx = sgen::gen_ctx_var("self", "prop");
        let child_nctx = sgen::gen_ctx_var("child", "node");

        Self {
            sgen,
            ident,
            generics,
            node_ctx: nctx,
            prop_ctx: pctx,
            child_node_ctx: child_nctx,
            var_defs: vec![],
            property_handlers: vec![],
            extra_properties_handler: None,
            node_handlers: vec![],
            field_values: vec![],
            extra_nodes_handler: None,
        }
    }

    pub fn build(self) -> TokenStream {
        let ty_node_ctx = self.sgen.ty_node_context();
        let ty_result = sgen::ty_result();

        let main_loop: syn::Stmt = {
            let prop_pattern = self.property_handlers.iter().map(|a| a.0.to_byte_str());
            let prop_expr = self.property_handlers.iter().map(|a| &a.1);
            let prop_extra = &self.extra_properties_handler;
            let child_pattern = self.node_handlers.iter().map(|a| a.0.to_byte_str());
            let child_expr = self.node_handlers.iter().map(|a| &a.1);
            let child_extra = &self.extra_nodes_handler;

            let nctx = &self.node_ctx;
            let pctx = &self.prop_ctx;
            let child_nctx = &self.child_node_ctx;
            let prop_name = sgen::gen_local_var("prop_name");
            let child_name = sgen::gen_local_var("child_name");
            parse_quote! {
                #ty_node_ctx::__read_item_with(
                    #nctx,
                    |#prop_name, mut #pctx| {
                        match #prop_name {
                            #( #prop_pattern => #prop_expr, )*
                            _ => { #prop_extra }
                        }
                        Ok(())
                    },
                    |#child_name, mut #child_nctx| {
                        match #child_name {
                            #( #child_pattern => #child_expr, )*
                            _ => { #child_extra }
                        }
                        Ok(())
                    },
                )?;
            }
        };

        let return_value: syn::Expr = {
            let field = self.field_values.iter().map(|a| &a.0);
            let value = self.field_values.iter().map(|a| &a.1);
            parse_quote! {
                Self {
                    #( #field: #value ),*
                }
            }
        };

        let (_impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let blob = self.sgen.blob();
        let found = self.generics.lifetimes().any(|lt| lt.lifetime == *blob);
        let new_generics = (!found).then(|| {
            let mut generics = self.generics.clone();
            generics
                .params
                .insert(0, syn::LifetimeParam::new(blob.clone()).into());
            generics
        });
        let (impl_generics, _ty_generics, _where_clause) = new_generics
            .as_ref()
            .map_or_else(|| self.generics.split_for_impl(), |g| g.split_for_impl());

        let trait_ = self.sgen.trait_deserialize_node_with_lifetime();
        let ty_ctx = self.sgen.ty_node_context_with_lifetime();
        let ty_error = self.sgen.ty_error();

        let ident = self.ident;
        let self_node_ctx = self.node_ctx;
        let local_var_defs = self.var_defs;

        quote! {
            #[automatically_derived]
            impl #impl_generics #trait_ for #ident #ty_generics #where_clause {
                fn deserialize_node(#self_node_ctx: &mut #ty_ctx) -> #ty_result<Self, #ty_error> {
                    #( #local_var_defs )*
                    #main_loop
                    Ok(#return_value)
                }
            }
        }
    }

    pub fn add_node_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: NodeSpec,
    ) {
        let NodeSpec { deserialize_with } = spec;

        let ty_node_ctx = self.sgen.ty_node_context();

        let deserialize_with =
            deserialize_with.unwrap_or_else(|| self.sgen.expr_node_deserializer(field_ty));

        let nctx = &self.node_ctx;
        let value = parse_quote! {
            #ty_node_ctx::deserialize_node_with(&#nctx, #deserialize_with)?
        };
        self.set_field_value(field_ident, value);
    }

    pub fn add_property_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: PropertySpec,
    ) -> Result<(), darling::Error> {
        let PropertySpec {
            name,
            fallback,
            deserialize_with,
            default,
        } = spec;

        let ty_node_ctx = self.sgen.ty_node_context();
        let ty_option = sgen::ty_option();

        let name = name.resolve(&field_ident)?;
        let name_str = name.to_str();
        let name_bytes = name.to_byte_str();
        let var_name = self.define_var(&field_ident, &sgen::expr_none(field_ty));

        let deserialize_with =
            deserialize_with.unwrap_or_else(|| self.sgen.expr_property_deserializer(field_ty));
        let update_var = |pctx: &syn::Ident| {
            let value = sgen::expr_some(&parse_quote! { (#deserialize_with)(&mut #pctx)? });
            parse_quote! { { #var_name = #value; } }
        };

        let mut field_value = match default {
            crate::meta::PropertyDefault::None => {
                let nctx = &self.node_ctx;
                parse_quote! { #var_name.ok_or_else(|| #ty_node_ctx::error_missing_property(&#nctx, #name_str))? }
            }
            crate::meta::PropertyDefault::DefaultTrait => {
                parse_quote! { #ty_option::unwrap_or_default(#var_name) }
            }
            crate::meta::PropertyDefault::Value(expr) => {
                parse_quote! { #ty_option::unwrap_or_else(#var_name, || #expr) }
            }
        };

        match fallback {
            Fallback::None => {}
            Fallback::Parent => {
                let nctx = &self.node_ctx;
                let parent_pctx = sgen::gen_ctx_var("parent", "prop");
                let update_stmt = update_var(&parent_pctx);
                field_value = parse_quote! {
                    {
                        if #var_name.is_none() {
                            #ty_node_ctx::__with_parent_property(#nctx, #name_bytes, |mut #parent_pctx| {
                                #update_stmt;
                                Ok(())
                            })?;
                        }
                        #field_value
                    }
                };
            }
        }

        self.add_property_handler(name, update_var);
        self.set_field_value(field_ident, field_value);
        Ok(())
    }

    pub fn add_extra_properties_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: ExtraPropertiesSpec,
    ) -> Result<(), darling::Error> {
        let ExtraPropertiesSpec { insert_with } = spec;

        let var_name = self.define_var(&field_ident, &sgen::expr_default(field_ty));

        let insert_with =
            insert_with.unwrap_or_else(|| self.sgen.expr_property_collection_inserter(field_ty));
        let update_var = |pctx: &syn::Ident| {
            parse_quote! { (#insert_with)(&mut #var_name, &mut #pctx)? }
        };

        self.add_extra_properties_handler(update_var)?;
        self.set_field_value(field_ident, parse_quote! { # var_name });
        Ok(())
    }

    pub fn add_child_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: ChildSpec,
    ) -> Result<(), darling::Error> {
        let ChildSpec {
            name,
            default,
            deserialize_with,
        } = spec;

        let ty_option = sgen::ty_option();

        let name = name.resolve(&field_ident)?;
        let name_str = name.to_str();
        let var_name = self.define_var(&field_ident, &sgen::expr_none(field_ty));

        let deserialize_with =
            deserialize_with.unwrap_or_else(|| self.sgen.expr_node_deserializer(field_ty));
        let update_var = |nctx: &syn::Ident| {
            let value = sgen::expr_some(&parse_quote! { (#deserialize_with)(&mut #nctx)? });
            parse_quote! { { #var_name = #value; } }
        };

        let field_value = if default {
            parse_quote! {
                #ty_option::unwrap_or_default(#var_name)
            }
        } else {
            let nctx = &self.node_ctx;
            parse_quote! {
                #ty_option::ok_or_else(#var_name, || #nctx.error_missing_child_node(#name_str))?
            }
        };

        self.add_child_handler(name, update_var);
        self.set_field_value(field_ident, field_value);
        Ok(())
    }

    pub fn add_repeated_children_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: RepeatedChildrenSpec,
    ) -> Result<(), darling::Error> {
        let RepeatedChildrenSpec { name, insert_with } = spec;

        let name = name.resolve(&field_ident)?;
        let var_name = self.define_var(&field_ident, &sgen::expr_default(field_ty));

        let insert_with =
            insert_with.unwrap_or_else(|| self.sgen.expr_node_collection_inserter(field_ty));
        let update_var = |nctx: &syn::Ident| {
            parse_quote! { (#insert_with)(&mut #var_name, &mut #nctx)? }
        };

        self.add_child_handler(name, update_var);
        self.set_field_value(field_ident, parse_quote! { #var_name });
        Ok(())
    }

    pub fn add_extra_children_field(
        &mut self,
        field_ident: FieldIdent,
        field_ty: &syn::Type,
        spec: ExtraChildrenSpec,
    ) -> Result<(), darling::Error> {
        let ExtraChildrenSpec { insert_with } = spec;

        let var_name = self.define_var(&field_ident, &sgen::expr_default(field_ty));

        let insert_with =
            insert_with.unwrap_or_else(|| self.sgen.expr_node_collection_inserter(field_ty));
        let update_var = |nctx: &syn::Ident| {
            parse_quote! { (#insert_with)(&mut #var_name, &mut #nctx)? }
        };

        self.add_extra_children_handler(update_var)?;
        self.set_field_value(field_ident, parse_quote! { #var_name });
        Ok(())
    }

    fn define_var(&mut self, field_ident: &FieldIdent, value: &syn::Expr) -> syn::Ident {
        let name = sgen::gen_field_var(field_ident);
        let var_decl = parse_quote! { let mut #name = #value; };
        self.var_defs.push(var_decl);
        name
    }

    fn add_property_handler<F>(&mut self, name: ResolvedName, handler: F)
    where
        F: for<'a> FnOnce(&'a syn::Ident) -> syn::Expr,
    {
        let pctx = &self.prop_ctx;
        self.property_handlers.push((name, handler(pctx)));
    }

    fn add_extra_properties_handler<F>(&mut self, handler: F) -> Result<(), darling::Error>
    where
        F: for<'a> FnOnce(&'a syn::Ident) -> syn::Expr,
    {
        if self.extra_properties_handler.is_some() {
            return Err(darling::Error::custom(
                "only one field can be marked as extra_properties",
            ));
        }
        let pctx = &self.prop_ctx;
        self.extra_properties_handler = Some(handler(pctx));
        Ok(())
    }

    fn add_child_handler<F>(&mut self, name: ResolvedName, handler: F)
    where
        F: for<'a> FnOnce(&'a syn::Ident) -> syn::Expr,
    {
        let child_nctx = &self.child_node_ctx;
        self.node_handlers.push((name, handler(child_nctx)));
    }

    fn set_field_value(&mut self, field_ident: FieldIdent, value: syn::Expr) {
        self.field_values.push((field_ident, value));
    }

    fn add_extra_children_handler<F>(&mut self, handler: F) -> Result<(), darling::Error>
    where
        F: for<'a> FnOnce(&'a syn::Ident) -> syn::Expr,
    {
        if self.extra_nodes_handler.is_some() {
            return Err(darling::Error::custom(
                "only one field can be marked as extra_nodes",
            ));
        }
        let child_nctx = &self.child_node_ctx;
        self.extra_nodes_handler = Some(handler(child_nctx));
        Ok(())
    }
}
