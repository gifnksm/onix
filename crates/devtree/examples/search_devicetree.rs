use core::fmt;
use std::{
    fs, iter,
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use argh::FromArgs;
use devtree::{
    Devicetree,
    blob::Node,
    tree_cursor::{TreeCursor, TreeCursorAllocExt as _, TreeIterator as _},
    types::property::Phandle,
};
use snafu::ResultExt as _;
use snafu_utils::{GenericError, Report};

/// Search devicetree nodes and prints them to stdout.
#[derive(Debug, FromArgs)]
struct Args {
    #[argh(positional)]
    glob: String,
    #[argh(positional)]
    blob_path: Vec<PathBuf>,

    /// print type (`full_name`, `path` or `tree`)
    #[argh(option, default = "Default::default()")]
    print: Print,
    /// treat glob as a phandle value (u32)
    #[argh(switch)]
    phandle: bool,
}

#[derive(Debug, Default)]
enum Print {
    #[default]
    FullName,
    Path,
    Tree,
}

impl FromStr for Print {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full_name" => Ok(Self::FullName),
            "path" => Ok(Self::Path),
            "tree" => Ok(Self::Tree),
            _ => Err("invalid value".into()),
        }
    }
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
        search_blob_by_glob(args, blob_path).with_whatever_context(|_| {
            format!(
                "failed to search devicetree blob nodes, path={}",
                blob_path.display()
            )
        })?;
    }

    Ok(())
}

fn search_blob_by_glob(args: &Args, path: &Path) -> Result<(), GenericError> {
    let blob = fs::read(path).whatever_context("failed to open devicetree blob")?;
    let dt = Devicetree::from_bytes(&blob).whatever_context("failed to read devicetree blob")?;
    let mut tree_cursor = dt.tree_cursor();

    println!("Devicetree: {}", path.display());

    if args.phandle {
        let phandle = u32::from_str(&args.glob).with_whatever_context(|_| {
            format!("failed to parse phandle value, phandle={}", args.glob)
        })?;
        println!("Phandle: {phandle}");

        let found = tree_cursor
            .read_node_by_phandle(Phandle::new(phandle))
            .whatever_context("failed to read devicetree blob")?;
        if let Some(node) = found {
            println!("{}", PrintNode::new(args, &node, &tree_cursor));
        } else {
            println!("No node found with phandle {phandle}");
        }
    } else {
        println!("Glob: {:?}", args.glob);

        let mut iter = tree_cursor.read_descendant_nodes_by_glob(args.glob.as_str());
        let mut index = 0_usize..;

        // Do NOT swap the order of zip here!
        // If you change the order, the index will advance even when iter does not yield
        // a node, leading to incorrect match counts.
        while let Some((node, i)) = iter::zip(iter.by_ref(), index.by_ref()).next() {
            let node = node.whatever_context("failed to read devicetree blob")?;
            println!("[{i}]: {}", PrintNode::new(args, &node, iter.tree_cursor()));
        }
        let found = index.next().unwrap();
        println!("{found} nodes found");
    }

    Ok(())
}

struct PrintNode<'a, 'blob, C> {
    args: &'a Args,
    node: &'a Node<'blob>,
    tree_cursor: &'a C,
}

impl<'blob, C> fmt::Display for PrintNode<'_, 'blob, C>
where
    C: TreeCursor<'blob> + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.args.print {
            Print::FullName => fmt::Debug::fmt(&self.node.full_name(), f),
            Print::Path => fmt::Debug::fmt(&self.tree_cursor.path(), f),
            Print::Tree => write!(f, "{:#?}", &self.tree_cursor.clone().debug_tree()),
        }
    }
}

impl<'a, 'blob, C> PrintNode<'a, 'blob, C> {
    fn new(args: &'a Args, node: &'a Node<'blob>, tree_cursor: &'a C) -> Self {
        Self {
            args,
            node,
            tree_cursor,
        }
    }
}
