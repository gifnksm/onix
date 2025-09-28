use devtree::{
    DeserializeNode, Devicetree,
    tree_cursor::TreeCursor as _,
    types::{ByteStr, ByteString},
};
use snafu::ResultExt as _;
use snafu_utils::GenericError;
use spin::Once;

#[derive(Debug, Default, DeserializeNode)]
struct ChosenNode<'blob> {
    // Optional properties with defaults.
    #[devtree(property(default))]
    pub bootargs: Option<&'blob ByteStr>,
    #[devtree(property(name = "stdout-path", default))]
    pub stdout_path: Option<&'blob ByteStr>,
    #[devtree(property(name = "stdin-path", default))]
    pub stdin_path: Option<&'blob ByteStr>,
}

struct Chosen {
    stdout_path: Option<ByteString>,
    stdin_path: Option<ByteString>,
}

static CHOSEN: Once<Chosen> = Once::new();

pub fn init(dt: &Devicetree) -> Result<(), GenericError> {
    let chosen = dt
        .tree_cursor()
        .read_node_by_path("/chosen")
        .whatever_context("failed to read chosen node")?
        .map(devtree::tree_cursor::NodeWithCursor::deserialize_node::<ChosenNode<'_>>)
        .transpose()
        .whatever_context("failed to deserialize chosen node")?
        .unwrap_or_default();
    CHOSEN.call_once(|| Chosen {
        stdout_path: chosen.stdout_path.map(ByteString::from),
        stdin_path: chosen.stdin_path.map(ByteString::from),
    });
    Ok(())
}

pub fn stdout_path() -> Option<&'static ByteString> {
    let chosen = CHOSEN.get()?;
    chosen.stdout_path.as_ref()
}

pub fn stdin_path() -> Option<&'static ByteString> {
    let chosen = CHOSEN.get()?;
    chosen.stdin_path.as_ref().or_else(stdout_path)
}
