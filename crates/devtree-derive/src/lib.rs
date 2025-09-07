use darling::FromDeriveInput as _;
use proc_macro2::TokenStream;
use syn::{parse_macro_input, parse_quote};

use self::{
    builder::{Builder, FieldIdent},
    meta::{
        ChildSpec, Fallback, FieldSpec, Input, PropertySpec, RepeatedChildrenSpec, ResolvedName,
    },
    sgen::SymbolGenerator,
};

mod builder;
mod meta;
mod sgen;

/// Derive macro `#[derive(DeserializeNode)]` for deserializing devicetree
/// nodes into Rust structs.
///
/// # Restrictions
///
/// * Only supports structs. Enums are not supported.
/// * At most one field can be annotated with `#[devtree(extra_properties)]`.
/// * At most one field can be annotated with `#[devtree(extra_children)]`.
///
/// # Struct-level Attributes
///
/// Attributes that apply to the struct as a whole.
///
/// * `#[devtree(crate = path)]`
///
///   Specifies the path to the `devtree` crate. If omitted, it defaults to
///   `::devtree`.
///
/// * `#[devtree(blob_lifetime = "'name")]`
///
///   Specifies the lifetime parameter name to be used for fields that borrow
///   from the devicetree blob.
///   If omitted, the default lifetime `'blob` is used.
///
///   **Notes:**
///
///   * The specified lifetime must be written as a string literal containing
///     the leading `'`. For example:
///
///     ```rust
///     use devtree::{DeserializeNode, types::property::Model};
///
///     #[derive(DeserializeNode)]
///     #[devtree(blob_lifetime = "'dt")]
///     pub struct Root<'dt> {
///         #[devtree(property)]
///         pub model: Model<'dt>,
///     }
///     ```
///
///   * If the struct already declares a lifetime parameter, its name must match
///     the one specified by this attribute. Otherwise, a compile error occurs.
///
///   * Multiple lifetimes are not supported. Each struct can have only one
///     devicetree blob lifetime.
///
/// # Field-level Attributes
///
/// Attributes that apply to individual fields within the struct.
///
/// ## Node fields
///
/// * `#[devtree(node)]`
///
///   Deserialize the current node into the field.
///
///   Field types can be:
///
///   - [`NodeFullName`] — the node's full name (including unit address)
///   - [`NodeName`] — the node's base name (excluding unit address)
///   - [`NodeUnitAddress`] — the node's unit address
///   - Any type implementing [`DeserializeNode`] — deserialize the full node
///     into a custom type
///
///   **Optional parameters:**
///
///   - `deserialize_with = expr` — Custom deserializer function
///
/// ## Property fields
///
/// Attributes used to deserialize properties from a devicetree node.
///
/// * `#[devtree(property)]`
///
///   Deserialize the property with the same name as the field. The field type
///   must implement [`DeserializeProperty`].
///
///   **Forms:**
///
///   * `#[devtree(property)]` — use the field name as the property name.
///   * `#[devtree(property = "name")]` — explicitly specify the property name.
///   * `#[devtree(property(...))]` — specify additional options (see below).
///
///   **Optional parameters:**
///
///   * `name = "..."` — Deserialize the property with the specified `"name"`.
///   * `fallback = "parent"` — Fallback to the parent node if missing. Error if
///     parent node also lacks it.
///   * `default` — Use `Default::default()` if missing. Field type must
///     implement [`Default`].
///   * `default = value` — Use the specified `value` if missing.
///   * `deserialize_with = expr` — Use `expr` instead of
///     [`DeserializeProperty::deserialize_property`]. The `expr` must be
///     callable as `fn(&mut PropertyContext<'_, 'blob>) -> Result<T,
///     DeserializeError>` where `T` is field type.
///
///   **Combination behavior:**
///
///   * `fallback = "parent"` + `default` — Fallback to parent if missing; use
///     default if parent also lacks it.
///
/// * `#[devtree(extra_properties)]`
///
///   Collects remaining properties that do not match other fields. The field
///   type must implement [`PropertyCollection`].
///   Only one field per struct may have this attribute.
///
///   **Optional parameters:**
///
///   * `insert_with = expr` — Use `expr` instead of
///     [`PropertyCollection::insert_property`]. The `expr` must be callable as
///     `fn(&mut PropertyContext<'_, 'blob>) -> Result<T, DeserializeError>`
///     where `T` is field type.
///
/// ## Child node fields
///
/// Attributes used to deserialize child nodes of the current node.
///
/// * `#[devtree(child)]`
///
///   Deserialize a single child node. The field type must implement
///   [`DeserializeNode`].
///
///   **Forms:**
///
///   * `#[devtree(child)]` — use the field name as the node name.
///   * `#[devtree(child(name = "name"))]` — specify the node name explicitly.
///   * `#[devtree(child(...))]` — specify additional options.
///
///   **Optional parameters:**
///
///   * `name = "..."` — Deserialize the child node with the given `"name"`.
///   * `default` — Use [`Default::default()`] if the node is missing.
///   * `deserialize_with = expr` — Use `expr` instead of
///     [`DeserializeNode::deserialize_node`]. The `expr` must be callable as
///     `fn(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>` where
///     `T` is field type.
///
/// * `#[devtree(repeated_children)]`
///
///   Collect multiple child nodes with the same name. The field type must
///   implement [`NodeCollection`]. Works even if only one node exists.
///
///   **Optional parameters:**
///
///   * `name = "..."` — Collect nodes with the specified `"name"`.
///   * `insert_with = expr` — Use `expr` instead of
///     [`NodeCollection::insert_node`]. The `expr` must be callable as `fn(&mut
///     NodeContext<'_, 'blob>) -> Result<T, DeserializeError>` where `T` is
///     field type.
///
/// * `#[devtree(extra_children)]`
///
///   Collects remaining child nodes that do not match other fields. The field
///   type must implement [`NodeCollection`].
///   Only one field per struct may have this attribute.
///
///   **Optional parameters:**
///
///   * `insert_with = expr` — Use `expr` instead of
///     [`NodeCollection::insert_node`]. The `expr` must be callable as `fn(&mut
///     NodeContext<'_, 'blob>) -> Result<T, DeserializeError>` where `T` is
///     field type.
///
/// [`NodeFullName`]: ::devtree::types::node::NodeFullName
/// [`NodeName`]: ::devtree::types::node::NodeName
/// [`NodeUnitAddress`]: ::devtree::types::node::NodeUnitAddress
/// [`DeserializeProperty`]: ::devtree::de::DeserializeProperty
/// [`DeserializeProperty::deserialize_property`]: ::devtree::de::DeserializeProperty::deserialize_property
/// [`PropertyCollection`]: ::devtree::de::PropertyCollection
/// [`PropertyCollection::insert_property`]: ::devtree::de::PropertyCollection::insert_property
/// [`DeserializeNode`]: ::devtree::de::DeserializeNode
/// [`DeserializeNode::deserialize_node`]: ::devtree::de::DeserializeNode::deserialize_node
/// [`NodeCollection`]: ::devtree::de::NodeCollection
/// [`NodeCollection::insert_node`]: ::devtree::de::NodeCollection::insert_node
///
/// # Example
///
/// The following example demonstrates how to implement the key requirements
/// defined in section *3. Device Node Requirements* of [the Devicetree
/// Specification v0.4]. It also shows how to use various features of the
/// derive macro, such as handling properties, child nodes, defaults, and
/// fallbacks.
///
/// This example is illustrative and does not cover every devicetree
/// schema detail. Instead, it highlights the most common patterns.
///
/// [the Devicetree Specification v0.4]: https://github.com/devicetree-org/devicetree-specification/releases/tag/v0.4
///
/// ```rust
/// use std::collections::BTreeMap;
///
/// use devtree::{
///     DeserializeNode,
///     de::util,
///     types::{
///         ByteStr,
///         node::{NodeFullName, NodeUnitAddress},
///         property::{AddressCells, Compatible, Model, Ranges, Reg, SizeCells, Status},
///     },
/// };
///
/// // Root node demonstrating top-level properties and child nodes.
/// #[derive(Debug, DeserializeNode)]
/// pub struct Root<'blob> {
///     // Required properties.
///     #[devtree(property = "#address-cells")]
///     pub address_cells: AddressCells,
///     #[devtree(property = "#size-cells")]
///     pub size_cells: SizeCells,
///     #[devtree(property)]
///     pub model: Model<'blob>,
///     #[devtree(property)]
///     pub compatible: Compatible<'blob>,
///
///     // Optional properties with defaults.
///     #[devtree(property(name = "serial-number", default))]
///     pub serial_number: Option<&'blob ByteStr>,
///     #[devtree(property(name = "chassis-type", default))]
///     pub chassis_type: Option<&'blob ByteStr>,
///
///     // Child nodes demonstrating different attribute forms:
///     // - aliases: custom deserialization into a PropertyCollection (BTreeMap)
///     #[devtree(child(
///         default,
///         deserialize_with = util::deserialize_node_as_property_collection
///     ))]
///     pub aliases: BTreeMap<&'blob ByteStr, &'blob ByteStr>,
///
///     // - memory: repeated children nodes
///     #[devtree(repeated_children)]
///     pub memory: Vec<Memory<'blob>>,
///
///     // - reserved_memory: optional child node with default
///     #[devtree(child(name = "reserved-memory", default))]
///     pub reserved_memory: Option<ReservedMemory<'blob>>,
///
///     // - chosen: optional child node with default
///     #[devtree(child(default))]
///     pub chosen: Option<Chosen<'blob>>,
///
///     // - cpus: required child node
///     #[devtree(child)]
///     pub cpus: Cpus<'blob>,
/// }
///
/// // Memory node demonstrating unit address and required properties.
/// #[derive(Debug, DeserializeNode)]
/// pub struct Memory<'blob> {
///     // Node unit address.
///     #[devtree(node)]
///     pub unit_address: NodeUnitAddress<'blob>,
///
///     // Required properties.
///     #[devtree(property)]
///     pub device_type: &'blob ByteStr,
///     #[devtree(property)]
///     pub reg: Reg<'blob>,
/// }
///
/// // ReservedMemory node demonstrating extra_children to collect unmatched
/// // children.
/// #[derive(Debug, DeserializeNode)]
/// pub struct ReservedMemory<'blob> {
///     // Required properties.
///     #[devtree(property = "#address-cells")]
///     pub address_cells: AddressCells,
///     #[devtree(property = "#size-cells")]
///     pub size_cells: SizeCells,
///     #[devtree(property)]
///     pub ranges: Ranges<'blob>,
///
///     // Extra children collected in a BTreeMap.
///     #[devtree(extra_children)]
///     pub children: BTreeMap<NodeFullName<'blob>, ReservedMemoryChild<'blob>>,
/// }
///
/// // Child nodes under "reserved-memory".
/// #[derive(Debug, DeserializeNode)]
/// pub struct ReservedMemoryChild<'blob> {
///     // Node full name.
///     #[devtree(node)]
///     pub full_name: NodeFullName<'blob>,
///
///     // Optional properties with defaults.
///     #[devtree(property(default))]
///     pub reg: Option<Reg<'blob>>,
/// }
///
/// // Optional chosen node demonstrating default properties.
/// #[derive(Debug, DeserializeNode)]
/// pub struct Chosen<'blob> {
///     // Optional properties with defaults.
///     #[devtree(property(default))]
///     pub bootargs: Option<&'blob ByteStr>,
///     #[devtree(property(name = "stdout-path", default))]
///     pub stdout_path: Option<&'blob ByteStr>,
///     #[devtree(property(name = "stdin-path", default))]
///     pub stdin_path: Option<&'blob ByteStr>,
/// }
///
/// // CPUs node demonstrating repeated children.
/// #[derive(Debug, DeserializeNode)]
/// pub struct Cpus<'blob> {
///     // Repeated child nodes.
///     #[devtree(property = "#address-cells")]
///     pub address_cells: AddressCells,
///     #[devtree(property = "#size-cells")]
///     pub size_cells: SizeCells,
///
///     // Repeated child nodes with the same name.
///     #[devtree(repeated_children)]
///     pub cpu: Vec<Cpu<'blob>>,
/// }
///
/// // CPU node demonstrating fallback, default, and custom deserialization.
/// #[derive(Debug, DeserializeNode)]
/// pub struct Cpu<'blob> {
///     // Node unit address.
///     #[devtree(node)]
///     pub unit_address: NodeUnitAddress<'blob>,
///
///     // Required properties.
///     #[devtree(property)]
///     pub device_type: &'blob ByteStr,
///     #[devtree(property)]
///     pub reg: Reg<'blob>,
///
///     // Optional properties with fallback, default, and custom deserialization.
///     #[devtree(property)]
///     pub compatible: Compatible<'blob>,
///     #[devtree(property(
///         name = "clock-frequency",
///         fallback = "parent",
///         default,
///         deserialize_with = |pctx| util::deserialize_u64_or_u32_property(pctx).map(Some),
///     ))]
///     pub clock_frequency: Option<u64>,
///     #[devtree(property(
///         name = "timebase-frequency",
///         fallback = "parent",
///         default,
///         deserialize_with = |pctx| util::deserialize_u64_or_u32_property(pctx).map(Some),
///     ))]
///     pub timebase_frequency: Option<u64>,
///
///     // Optional property with default.
///     #[devtree(property(default))]
///     pub status: Status,
/// }
/// ```
#[proc_macro_derive(DeserializeNode, attributes(devtree))]
pub fn derive_deserialize_node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    generate_impl(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn generate_impl(input: &syn::DeriveInput) -> Result<TokenStream, syn::Error> {
    let Input {
        data,
        ident,
        generics,
        crate_path: devtree,
        blob_lifetime,
    } = Input::from_derive_input(input)?;

    let devtree = devtree.unwrap_or_else(|| parse_quote! { ::devtree });
    let mut builder = Builder::new(devtree, blob_lifetime.0, ident, generics);

    let fields = data.take_struct().unwrap();

    for (i, field) in fields.fields.into_iter().enumerate() {
        let field_ident = match field.ident {
            Some(ident) => FieldIdent::Named(ident),
            None => FieldIdent::Unnamed(i),
        };
        let field_ty = field.ty;

        match field.field_spec {
            FieldSpec::Node(spec) => {
                builder.add_node_field(field_ident, &field_ty, spec);
            }
            FieldSpec::Property(spec) => {
                builder.add_property_field(field_ident, &field_ty, spec)?;
            }
            FieldSpec::ExtraProperties(spec) => {
                builder.add_extra_properties_field(field_ident, &field_ty, spec)?;
            }
            FieldSpec::Child(spec) => {
                builder.add_child_field(field_ident, &field_ty, spec)?;
            }
            FieldSpec::RepeatedChildren(spec) => {
                builder.add_repeated_children_field(field_ident, &field_ty, spec)?;
            }
            FieldSpec::ExtraChildren(spec) => {
                builder.add_extra_children_field(field_ident, &field_ty, spec)?;
            }
        }
    }

    let output = builder.build();
    Ok(output)
}
