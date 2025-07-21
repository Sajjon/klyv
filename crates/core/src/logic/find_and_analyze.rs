pub use crate::prelude::*;

#[builder]
pub fn find_in(path: impl AsRef<Path>) -> Result<Tree> {
    let mut tree = Tree::Root {
        path: path.as_ref().to_path_buf(),
    };
    _find_in(&mut tree)?;
    Ok(tree)
}

#[builder]
pub fn analyze_file(file: syn::File) -> Result<Vec<SourceItem>> {
    file.items
        .into_iter()
        .map(SourceItem::try_from)
        .collect::<Result<Vec<SourceItem>>>()
}
