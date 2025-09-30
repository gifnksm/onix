use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use argh::FromArgs;
use devtree::{
    Devicetree,
    blob::Item,
    tree_cursor::{TreeCursor as _, TreeCursorAllocExt as _, TreeIterator as _},
};
use snafu::ResultExt as _;
use snafu_utils::{GenericError, Report};

/// Iterate over devicetree items and prints them to stdout.
#[derive(Debug, FromArgs)]
#[expect(clippy::struct_excessive_bools)]
struct Args {
    #[argh(positional)]
    blob_path: Vec<PathBuf>,

    /// print items
    #[argh(switch, short = 'i')]
    items: bool,

    /// print properties
    #[argh(switch, short = 'p')]
    properties: bool,

    /// print child nodes
    #[argh(switch, short = 'n')]
    children: bool,

    /// print descendant items
    #[argh(switch, short = 'I')]
    descendant_items: bool,

    /// print descendant properties
    #[argh(switch, short = 'P')]
    descendant_properties: bool,

    /// print descendant nodes
    #[argh(switch, short = 'N')]
    descendant_nodes: bool,
}

fn main() {
    let args: Args = argh::from_env();

    if let Err(err) = run(&args) {
        let report = Report::new(err);
        eprintln!("{report}");
        process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), GenericError> {
    for blob_path in &args.blob_path {
        iterate_blob(args, blob_path).with_whatever_context(|_| {
            format!(
                "failed to iterate devicetree blob items, path={}",
                blob_path.display()
            )
        })?;
    }

    Ok(())
}

fn iterate_blob(args: &Args, path: &Path) -> Result<(), GenericError> {
    let blob = fs::read(path).whatever_context("failed to open devicetree blob")?;
    let dt = Devicetree::from_bytes(&blob).whatever_context("failed to read devicetree blob")?;

    println!("Devicetree: {}", path.display());

    if args.items {
        println!("  Items:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        let mut iter = cursor.read_items();
        while let Some(item) = iter.next() {
            let item = item.whatever_context("failed to read devicetree blob")?;
            match item {
                Item::Property(property) => {
                    println!("      {}", property.name());
                }
                Item::Node(_node) => {
                    let cursor = iter.tree_cursor();
                    println!("    {}", cursor.path());
                }
            }
        }
    }

    if args.properties {
        println!("  Properties:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        for property in cursor.read_properties() {
            let property = property.whatever_context("failed to read devicetree blob")?;
            println!("    {}", property.name());
        }
    }

    if args.children {
        println!("  Children:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        let mut iter = cursor.read_children();
        while let Some(_) = iter.next() {
            let cursor = iter.tree_cursor();
            println!("    {}", cursor.path());
        }
    }

    if args.descendant_items {
        println!("  Descendant Items:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        let mut iter = cursor.read_descendant_items();
        while let Some(item) = iter.next() {
            let item = item.whatever_context("failed to read devicetree blob")?;
            match item {
                Item::Property(property) => {
                    println!("      {}", property.name());
                }
                Item::Node(_node) => {
                    let cursor = iter.tree_cursor();
                    println!("    {}", cursor.path());
                }
            }
        }
    }

    if args.descendant_properties {
        println!("  Descendant Properties:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        for property in cursor.read_descendant_properties() {
            let property = property.whatever_context("failed to read devicetree blob")?;
            println!("    {}", property.name());
        }
    }

    if args.descendant_nodes {
        println!("  Descendant Nodes:");
        let mut cursor = dt
            .tree_cursor()
            .whatever_context("failed to create tree cursor")?;
        let mut iter = cursor.read_descendant_nodes();
        while let Some(_) = iter.next() {
            let cursor = iter.tree_cursor();
            println!("    {}", cursor.path());
        }
    }

    Ok(())
}
