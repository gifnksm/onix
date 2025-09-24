use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use argh::FromArgs;
use devtree::{Devicetree, tree_cursor::TreeCursor as _};
use snafu::ResultExt as _;
use snafu_utils::{GenericError, Report};

/// Dump devicetree to stdout.
#[derive(Debug, FromArgs)]
struct Args {
    #[argh(positional)]
    blob_path: Vec<PathBuf>,
    /// enable pretty print
    #[argh(switch)]
    pretty: bool,
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
        dump_blob(args, blob_path).with_whatever_context(|_| {
            format!(
                "failed to dump devicetree blob, path={}",
                blob_path.display()
            )
        })?;
    }

    Ok(())
}

fn dump_blob(args: &Args, path: &Path) -> Result<(), GenericError> {
    let blob = fs::read(path).whatever_context("failed to open devicetree blob")?;
    let dt = Devicetree::from_bytes(&blob).whatever_context("failed to read devicetree blob")?;

    println!("Devicetree: {}", path.display());

    let mut root = dt.tree_cursor();
    let debug_tree = root.debug_tree();
    if args.pretty {
        println!("{debug_tree:#?}");
    } else {
        println!("{debug_tree:?}");
    }

    Ok(())
}
