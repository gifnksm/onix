use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

use crate::{
    Fallback, FieldSpec, ResolvedName, SymbolGenerator,
    meta::{InputField, PropertyDefault},
    sgen,
};

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
    fields: Vec<Field>,
}

impl Builder {
    pub fn new(
        devtree: syn::Path,
        blob: syn::Lifetime,
        ident: syn::Ident,
        generics: syn::Generics,
    ) -> Self {
        let sgen = SymbolGenerator::new(devtree, blob);

        Self {
            sgen,
            ident,
            generics,
            fields: vec![],
        }
    }

    pub fn push_field(&mut self, i: usize, field: InputField) {
        self.fields.push(Field::new(i, field));
    }

    pub fn build(self) -> Result<TokenStream, darling::Error> {
        let lt_blob = self.sgen.lt_blob();
        let lt_de = sgen::gen_lt("de");
        let var_de = sgen::gen_var("de");

        let mut body = vec![];
        self.gen_var_defs(&var_de, &mut body)?;
        self.gen_with_items(&var_de, &mut body)?;
        self.gen_return_value(&var_de, &mut body)?;
        let ts = self.gen_impl(lt_blob, &lt_de, &var_de, &body);
        Ok(ts)
    }

    fn gen_var_defs(
        &self,
        var_de: &syn::Ident,
        body: &mut Vec<syn::Stmt>,
    ) -> Result<(), darling::Error> {
        for field in &self.fields {
            if let Some(stmt) = field.var_def(&self.sgen, var_de)? {
                body.push(stmt);
            }
        }
        Ok(())
    }

    fn gen_with_items(
        &self,
        var_de: &syn::Ident,
        body: &mut Vec<syn::Stmt>,
    ) -> Result<(), darling::Error> {
        let private = self.sgen.private();
        let var_sub_de = sgen::gen_var("sub_de");
        let mut prop_patterns = vec![];
        let mut prop_handlers = vec![];
        let mut extra_properties_handler = None;
        let mut child_patterns = vec![];
        let mut child_handlers = vec![];
        let mut extra_children_handler = None;
        for field in &self.fields {
            if let Some((name, handler)) = field.prop_handler(&self.sgen, &var_sub_de)? {
                prop_patterns.push(name.to_lit_byte_str());
                prop_handlers.push(handler);
            }
            if let Some(handler) = field.extra_properties_handler(&self.sgen, &var_sub_de) {
                if extra_properties_handler.is_some() {
                    return Err(darling::Error::custom(
                        "only one field can be marked as extra_properties",
                    ));
                }
                extra_properties_handler = Some(handler);
            }
            if let Some((name, handler)) = field.child_handler(&self.sgen, &var_sub_de)? {
                child_patterns.push(name.to_lit_byte_str());
                child_handlers.push(handler);
            }
            if let Some((name, handler)) =
                field.repeaded_children_handler(&self.sgen, &var_sub_de)?
            {
                child_patterns.push(name.to_lit_byte_str());
                child_handlers.push(handler);
            }
            if let Some(handler) = field.extra_children_handler(&self.sgen, &var_sub_de) {
                if extra_children_handler.is_some() {
                    return Err(darling::Error::custom(
                        "only one field can be marked as extra_children",
                    ));
                }
                extra_children_handler = Some(handler);
            }
        }

        let stmt = parse_quote! {
            #private::node_de_with_items(
                #var_de,
                |mut #var_sub_de| {
                    match #private::prop_de_name(&#var_sub_de) {
                        #( #prop_patterns => { #prop_handlers }, )*
                        _ => { #extra_properties_handler }
                    }
                    #private::Result::Ok(())
                },
                |mut #var_sub_de| {
                    match #private::node_de_name(&#var_sub_de) {
                        #( #child_patterns => { #child_handlers }, )*
                        _ => { #extra_children_handler }
                    }
                    #private::Result::Ok(())
                },
            )?;
        };
        body.push(stmt);
        Ok(())
    }

    fn gen_return_value(
        &self,
        var_de: &syn::Ident,
        body: &mut Vec<syn::Stmt>,
    ) -> Result<(), darling::Error> {
        let private = self.sgen.private();
        let mut idents = vec![];
        let mut values = vec![];
        for field in &self.fields {
            let value = field.field_value(&self.sgen, var_de)?;
            idents.push(field.ident.clone());
            values.push(value);
        }
        let value = parse_quote! {
            #private::Result::Ok(Self {
                #( #idents: (#values), )*
            })
        };
        body.push(syn::Stmt::Expr(value, None));
        Ok(())
    }

    fn gen_impl(
        &self,
        lt_blob: &syn::Lifetime,
        lt_de: &syn::Lifetime,
        var_de: &syn::Ident,
        body: &[syn::Stmt],
    ) -> TokenStream {
        let private = self.sgen.private();
        let gp_d = sgen::gen_generic_param_d();
        let ident = &self.ident;

        let (_impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let found = self.generics.lifetimes().any(|lt| lt.lifetime == *lt_blob);
        let new_generics = (!found).then(|| {
            let mut generics = self.generics.clone();
            generics
                .params
                .insert(0, syn::LifetimeParam::new(lt_blob.clone()).into());
            generics
        });
        let (impl_generics, _ty_generics, _where_clause) = new_generics
            .as_ref()
            .map_or_else(|| self.generics.split_for_impl(), |g| g.split_for_impl());

        quote! {
            #[automatically_derived]
            impl #impl_generics #private::DeserializeNode<#lt_blob> for #ident #ty_generics #where_clause {
                fn deserialize_node<#lt_de, #gp_d>(#var_de: &mut #gp_d) -> #private::Result<Self, #private::DeserializeError>
                    where
                        #gp_d: #private::NodeDeserializer<#lt_de, #lt_blob> + ?#private::Sized,
                {
                    #( #body )*
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldIdent {
    Named(syn::Ident),
    Unnamed(usize),
}

#[derive(Debug)]
struct Field {
    ident: FieldIdent,
    ty: syn::Type,
    spec: FieldSpec,
    var_name: syn::Ident,
}

impl Field {
    fn new(i: usize, input: InputField) -> Self {
        let ident = match input.ident {
            Some(ident) => FieldIdent::Named(ident),
            None => FieldIdent::Unnamed(i),
        };
        let var_name = sgen::gen_field_var(&ident);
        Self {
            ident,
            ty: input.ty,
            spec: input.field_spec,
            var_name,
        }
    }

    fn var_def(
        &self,
        sgen: &SymbolGenerator,
        de: &syn::Ident,
    ) -> Result<Option<syn::Stmt>, darling::Error> {
        let private = sgen.private();
        let ty = &self.ty;
        let value: syn::Expr = match &self.spec {
            FieldSpec::Node(_) => return Ok(None),
            FieldSpec::Property(spec) => {
                let prop_name = spec.name.resolve(&self.ident)?.to_lit_str();
                parse_quote! {  #private::PropertyCell::<#ty>::new(#de, #prop_name)? }
            }
            FieldSpec::Child(spec) => {
                let child_name = spec.name.resolve(&self.ident)?.to_lit_str();
                parse_quote! {  #private::NodeCell::<#ty>::new(#de, #child_name)? }
            }
            FieldSpec::ExtraProperties(_)
            | FieldSpec::RepeatedChildren(_)
            | FieldSpec::ExtraChildren(_) => parse_quote! { <#ty as #private::Default>::default() },
        };
        let var_name = &self.var_name;
        Ok(Some(parse_quote! { let mut #var_name = #value; }))
    }

    fn prop_handler(
        &self,
        sgen: &SymbolGenerator,
        var_sub_de: &syn::Ident,
    ) -> Result<Option<(ResolvedName, syn::Expr)>, darling::Error> {
        let private = sgen.private();
        let ty = &self.ty;
        match &self.spec {
            FieldSpec::Property(spec) => {
                let deserialize_with = spec.deserialize_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::DeserializeProperty>::deserialize_property }
                });
                let var_name = &self.var_name;
                let expr = parse_quote! {
                    #private::PropertyCell::set(&mut #var_name, (#deserialize_with)(&mut #var_sub_de)?)?
                };
                let prop_name = spec.name.resolve(&self.ident)?;
                Ok(Some((prop_name, expr)))
            }
            FieldSpec::ExtraProperties(_)
            | FieldSpec::Node(_)
            | FieldSpec::Child(_)
            | FieldSpec::RepeatedChildren(_)
            | FieldSpec::ExtraChildren(_) => Ok(None),
        }
    }

    fn extra_properties_handler(
        &self,
        sgen: &SymbolGenerator,
        var_sub_de: &syn::Ident,
    ) -> Option<syn::Expr> {
        let private = sgen.private();
        let ty = &self.ty;
        match &self.spec {
            FieldSpec::ExtraProperties(spec) => {
                let insert_with = spec.insert_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::PropertyCollection>::insert_property }
                });
                let var_name = &self.var_name;
                let expr = parse_quote! {
                    (#insert_with)(&mut #var_name, &mut #var_sub_de)?
                };
                Some(expr)
            }
            FieldSpec::Property(_)
            | FieldSpec::Node(_)
            | FieldSpec::Child(_)
            | FieldSpec::RepeatedChildren(_)
            | FieldSpec::ExtraChildren(_) => None,
        }
    }

    fn child_handler(
        &self,
        sgen: &SymbolGenerator,
        var_sub_de: &syn::Ident,
    ) -> Result<Option<(ResolvedName, syn::Expr)>, darling::Error> {
        let private = sgen.private();
        let ty = &self.ty;
        match &self.spec {
            FieldSpec::Child(spec) => {
                let deserialize_with = spec.deserialize_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::DeserializeNode>::deserialize_node }
                });
                let var_name = &self.var_name;
                let expr = parse_quote! {
                    #private::NodeCell::set(&mut #var_name, (#deserialize_with)(&mut #var_sub_de)?)?
                };
                let node_name = spec.name.resolve(&self.ident)?;
                Ok(Some((node_name, expr)))
            }
            FieldSpec::Node(_)
            | FieldSpec::Property(_)
            | FieldSpec::ExtraProperties(_)
            | FieldSpec::RepeatedChildren(_)
            | FieldSpec::ExtraChildren(_) => Ok(None),
        }
    }

    fn repeaded_children_handler(
        &self,
        sgen: &SymbolGenerator,
        var_sub_de: &syn::Ident,
    ) -> Result<Option<(ResolvedName, syn::Expr)>, darling::Error> {
        let private = sgen.private();
        let ty = &self.ty;
        match &self.spec {
            FieldSpec::RepeatedChildren(spec) => {
                let insert_with = spec.insert_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::NodeCollection>::insert_node }
                });
                let var_name = &self.var_name;
                let expr = parse_quote! {
                    (#insert_with)(&mut #var_name, &mut #var_sub_de)?
                };
                let node_name = spec.name.resolve(&self.ident)?;
                Ok(Some((node_name, expr)))
            }
            FieldSpec::Node(_)
            | FieldSpec::Property(_)
            | FieldSpec::ExtraProperties(_)
            | FieldSpec::Child(_)
            | FieldSpec::ExtraChildren(_) => Ok(None),
        }
    }

    fn extra_children_handler(
        &self,
        sgen: &SymbolGenerator,
        var_sub_de: &syn::Ident,
    ) -> Option<syn::Expr> {
        let private = sgen.private();
        let ty = &self.ty;
        match &self.spec {
            FieldSpec::ExtraChildren(spec) => {
                let insert_with = spec.insert_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::NodeCollection>::insert_node }
                });
                let var_name = &self.var_name;
                let expr = parse_quote! {
                    (#insert_with)(&mut #var_name, &mut #var_sub_de)?
                };
                Some(expr)
            }
            FieldSpec::Property(_)
            | FieldSpec::ExtraProperties(_)
            | FieldSpec::Node(_)
            | FieldSpec::Child(_)
            | FieldSpec::RepeatedChildren(_) => None,
        }
    }

    fn field_value(
        &self,
        sgen: &SymbolGenerator,
        var_de: &syn::Ident,
    ) -> Result<syn::Expr, darling::Error> {
        let private = sgen.private();
        let ty = &self.ty;
        let field_value = match &self.spec {
            FieldSpec::Node(spec) => {
                let deserialize_with = spec.deserialize_with.clone().unwrap_or_else(|| {
                    parse_quote! { <#ty as #private::DeserializeNode>::deserialize_node }
                });
                let var_cursor = sgen::gen_var("cursor");
                let var_node = sgen::gen_var("node");
                let var_sub_de = sgen::gen_var("sub_de");
                parse_quote! {
                    {
                        let mut #var_cursor = #private::NodeDeserializer::clone_tree_cursor(#var_de)?;
                        let mut #var_node = #private::TreeCursor::read_node(&mut #var_cursor);
                        let mut #var_sub_de =  #private::TreeNodeRef::node_deserializer(&mut #var_node);
                        (#deserialize_with)(&mut #var_sub_de)?
                    }
                }
            }
            FieldSpec::Property(spec) => {
                let var_name = &self.var_name;
                let mut field_value = match &spec.default {
                    PropertyDefault::None => {
                        parse_quote! { #private::PropertyCell::finish(#var_name)? }
                    }
                    PropertyDefault::DefaultTrait => {
                        parse_quote! { #private::PropertyCell::finish_or_default(#var_name) }
                    }
                    PropertyDefault::Value(expr) => {
                        parse_quote! { #private::PropertyCell::finish_or_else(#var_name, || { #expr }) }
                    }
                };
                match &spec.fallback {
                    Fallback::None => {}
                    Fallback::Parent => {
                        let deserialize_with = spec.deserialize_with.clone().unwrap_or_else(|| {
                            parse_quote! {
                                <#ty as #private::DeserializeProperty>::deserialize_property
                            }
                        });
                        let prop_name = spec.name.resolve(&self.ident)?;
                        let prop_name = prop_name.to_lit_byte_str();
                        let var_cursor = sgen::gen_var("cursor");
                        let var_parent = sgen::gen_var("parent");
                        let var_parent_de = sgen::gen_var("parent_de");
                        let var_parent_sub_de = sgen::gen_var("parent_sub_de");
                        field_value = parse_quote! {
                            {
                                if !#private::PropertyCell::has_value(&#var_name) {
                                    let mut #var_cursor = #private::NodeDeserializer::clone_tree_cursor(#var_de)?;
                                    if let #private::Option::Some(mut #var_parent) = #private::TreeCursor::read_parent(&mut #var_cursor) {
                                        let mut #var_parent_de = #private::TreeNodeRef::node_deserializer(&mut #var_parent);
                                        while let Some(mut #var_parent_de) = #private::NodeDeserializer::read_item(&mut #var_parent_de)? {
                                            if let #private::ItemDeserializer::Property(mut #var_parent_sub_de) = #var_parent_de {
                                                if #private::prop_de_name(&#var_parent_sub_de) == #prop_name {
                                                    #private::PropertyCell::set(&mut #var_name, (#deserialize_with)(&mut #var_parent_sub_de)?)?;
                                                    break;
                                                }
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                #field_value
                            }
                        };
                    }
                }
                field_value
            }
            FieldSpec::ExtraProperties(_spec) => {
                let var_name = &self.var_name;
                parse_quote! { #var_name }
            }
            FieldSpec::Child(spec) => {
                let var_name = &self.var_name;
                match &spec.default {
                    false => {
                        parse_quote! { #private::NodeCell::finish(#var_name)? }
                    }
                    true => {
                        parse_quote! { #private::NodeCell::finish_or_default(#var_name) }
                    }
                }
            }
            FieldSpec::RepeatedChildren(_spec) => {
                let var_name = &self.var_name;
                parse_quote! { #var_name }
            }
            FieldSpec::ExtraChildren(_spec) => {
                let var_name = &self.var_name;
                parse_quote! { #var_name }
            }
        };
        Ok(field_value)
    }
}
