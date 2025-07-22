pub use crate::prelude::*;

#[builder]
fn do_parse_file(content: &str) -> Result<syn::File> {
    syn::parse_file(content).map_err(Error::from)
}

#[builder]
pub fn parse_file(content: impl AsRef<str>) -> Result<Vec<SourceItem>> {
    let file = do_parse_file().content(content.as_ref()).call()?;
    file.items
        .into_iter()
        .map(SourceItem::try_from)
        .collect::<Result<Vec<SourceItem>>>()
}
