use super::LintCtx;
use crate::cop::Cop;
use crate::parse::{self, codemap::CodeMap, source::SourceFile};
use anyhow::{Result, bail};

pub(super) fn validate(ctx: &LintCtx<'_>, bytes: &[u8]) -> Result<()> {
    let source = SourceFile::from_bytes(ctx.path, bytes.to_vec());
    let tree = parse::parse_ruby(&source)?;
    let mut diagnostics = Vec::new();
    crate::cop::lint::Syntax.check_source(
        &source,
        &tree,
        &CodeMap::default(),
        &ctx.config.cop_config("Lint/Syntax"),
        &mut diagnostics,
        None,
    );
    if crate::cop::lint::has_syntax_fatals(&diagnostics) {
        bail!(
            "Autocorrection would introduce invalid Ruby in {}: {}",
            ctx.path.display(),
            diagnostics[0].message
        );
    }
    Ok(())
}
