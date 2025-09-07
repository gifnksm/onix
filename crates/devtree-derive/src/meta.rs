use darling::{FromDeriveInput, FromField, FromMeta, ast::Data, util::Ignored};
use proc_macro2::Span;

use crate::FieldIdent;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(devtree), supports(struct_any))]
pub struct Input {
    pub data: Data<Ignored, InputField>,
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    #[darling(default, rename = "crate")]
    pub crate_path: Option<syn::Path>,
    #[darling(default)]
    pub blob_lifetime: Lifetime,
}

#[derive(Debug)]
pub struct Lifetime(pub syn::Lifetime);

impl FromMeta for Lifetime {
    fn from_value(value: &syn::Lit) -> Result<Self, darling::Error> {
        (|| {
            let syn::Lit::Str(s) = value else {
                return Err(darling::Error::unexpected_lit_type(value));
            };
            let value = s.value();
            if !value.starts_with('\'') {
                return Err(darling::Error::custom(format!(
                    "lifetime name must start with apostrophe as in \"'blob\", got {value:?}"
                )));
            }
            if value == "'" {
                return Err(darling::Error::custom("lifetime name must not be empty"));
            }
            Ok(Self(syn::Lifetime::new(&value, s.span())))
        })()
        .map_err(|e| e.with_span(value))
    }
}

impl Default for Lifetime {
    fn default() -> Self {
        Self(syn::Lifetime::new("'blob", Span::call_site()))
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(devtree))]
pub struct InputField {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
    #[darling(flatten)]
    pub field_spec: FieldSpec,
}

#[derive(Debug, FromMeta, PartialEq, Eq)]
pub enum FieldSpec {
    Node(NodeSpec),
    Property(PropertySpec),
    ExtraProperties(ExtraPropertiesSpec),
    Child(ChildSpec),
    RepeatedChildren(RepeatedChildrenSpec),
    ExtraChildren(ExtraChildrenSpec),
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()))]
pub struct NodeSpec {
    #[darling(default)]
    pub deserialize_with: Option<syn::Expr>,
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()), from_expr = Self::from_expr)]
pub struct PropertySpec {
    #[darling(default)]
    pub name: Name,
    #[darling(default)]
    pub fallback: Fallback,
    #[darling(default)]
    pub default: PropertyDefault,
    #[darling(default)]
    pub deserialize_with: Option<syn::Expr>,
}

impl PropertySpec {
    fn from_expr(expr: &syn::Expr) -> Result<Self, darling::Error> {
        let name = Name::from_expr(expr)?;
        Ok(Self {
            name,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()))]
pub struct ExtraPropertiesSpec {
    #[darling(default)]
    pub insert_with: Option<syn::Expr>,
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()), from_expr = Self::from_expr)]
pub struct ChildSpec {
    #[darling(default)]
    pub name: Name,
    #[darling(default)]
    pub default: bool,
    #[darling(default)]
    pub deserialize_with: Option<syn::Expr>,
}

impl ChildSpec {
    fn from_expr(expr: &syn::Expr) -> Result<Self, darling::Error> {
        let name = Name::from_expr(expr)?;
        Ok(Self {
            name,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()), from_expr = Self::from_expr)]
pub struct RepeatedChildrenSpec {
    pub name: Name,
    #[darling(default)]
    pub insert_with: Option<syn::Expr>,
}

impl RepeatedChildrenSpec {
    fn from_expr(expr: &syn::Expr) -> Result<Self, darling::Error> {
        let name = Name::from_expr(expr)?;
        Ok(Self {
            name,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
#[darling(from_word = || Ok(Default::default()))]
pub struct ExtraChildrenSpec {
    #[darling(default)]
    pub insert_with: Option<syn::Expr>,
}

#[derive(Debug, Default, FromMeta, PartialEq, Eq)]
pub enum Fallback {
    #[default]
    None,
    Parent,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum PropertyDefault {
    #[default]
    None,
    DefaultTrait,
    Value(syn::Expr),
}

impl FromMeta for PropertyDefault {
    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        Ok(Self::Value(expr.clone()))
    }

    fn from_word() -> darling::Result<Self> {
        Ok(Self::DefaultTrait)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum Name {
    #[default]
    FromField,
    Str(syn::LitStr),
}

impl FromMeta for Name {
    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        Ok(Self::Str(syn::LitStr::from_expr(expr)?))
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        Ok(Self::Str(syn::LitStr::new(value, Span::call_site())))
    }
}

impl Name {
    pub fn resolve(&self, field_name: &FieldIdent) -> Result<ResolvedName, darling::Error> {
        match self {
            Self::FromField => match field_name {
                FieldIdent::Named(field_name) => {
                    let s = syn::LitStr::new(&field_name.to_string(), field_name.span());
                    Ok(ResolvedName::Str(s))
                }
                FieldIdent::Unnamed(_) => Err(darling::Error::custom(
                    "tuple struct field must specify `#[devtree(property(name = \"...\")]` or \
                     `#[devtree(node(name = \"...\")]`",
                )),
            },
            Self::Str(s) => Ok(ResolvedName::Str(s.clone())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedName {
    Str(syn::LitStr),
}

impl ResolvedName {
    pub fn to_str(&self) -> syn::LitStr {
        match self {
            Self::Str(s) => s.clone(),
        }
    }

    pub fn to_byte_str(&self) -> syn::LitByteStr {
        let s = self.to_str();
        syn::LitByteStr::new(s.value().as_bytes(), s.span())
    }
}
