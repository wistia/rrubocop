//! Bundler/InsecureProtocolSource — deprecate :rubygems / optional http://.

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InsecureProtocolSource;

impl Cop for InsecureProtocolSource {
    fn name(&self) -> &'static str {
        "Bundler/InsecureProtocolSource"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn uses_line_phase(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_http = config.get_bool("AllowHttpProtocol", true);
        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();
            if !trimmed.starts_with("source ") && !trimmed.starts_with("source(") {
                continue;
            }
            let line_num = i + 1;
            flag_deprecated_syms(
                self,
                source,
                line_str,
                line_num,
                diagnostics,
                &mut corrections,
            );
            if !allow_http {
                fix_http(
                    self,
                    source,
                    line_str,
                    line_num,
                    '\'',
                    diagnostics,
                    &mut corrections,
                );
                fix_http(
                    self,
                    source,
                    line_str,
                    line_num,
                    '"',
                    diagnostics,
                    &mut corrections,
                );
            }
        }
    }
}

fn flag_deprecated_syms(
    cop: &InsecureProtocolSource,
    source: &SourceFile,
    line_str: &str,
    line_num: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    for sym in [":gemcutter", ":rubygems", ":rubyforge"] {
        let Some(col) = line_str.find(sym) else {
            continue;
        };
        let mut diag = cop.diagnostic(
            source,
            line_num,
            col,
            format!(
                "The source `{sym}` is deprecated because HTTP requests are insecure. Please change your source to 'https://rubygems.org' if possible, or 'http://rubygems.org' if not."
            ),
        );
        if push_replace(
            corrections,
            source,
            line_num,
            col,
            sym.len(),
            "'https://rubygems.org'",
            cop.name(),
        ) {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

fn fix_http(
    cop: &InsecureProtocolSource,
    source: &SourceFile,
    line_str: &str,
    line_num: usize,
    quote: char,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    let needle = format!("{quote}http://");
    let Some(col) = line_str.find(&needle) else {
        return;
    };
    let url = url_in_quotes(&line_str[col + 1..], quote);
    let https_url = url.replacen("http://", "https://", 1);
    let mut diag = cop.diagnostic(
        source,
        line_num,
        col,
        format!("Use `{https_url}` instead of `{url}`."),
    );
    if let Some(http_col) = line_str.find("http://")
        && push_replace(
            corrections,
            source,
            line_num,
            http_col,
            7,
            "https://",
            cop.name(),
        )
    {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn push_replace(
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    source: &SourceFile,
    line_num: usize,
    col: usize,
    len: usize,
    replacement: &str,
    cop_name: &'static str,
) -> bool {
    let Some(corr) = corrections.as_deref_mut() else {
        return false;
    };
    let Some(line_start) = source.line_col_to_offset(line_num, 0) else {
        return false;
    };
    corr.push(crate::correction::Correction {
        start: line_start + col,
        end: line_start + col + len,
        replacement: replacement.to_string(),
        cop_name,
        cop_index: 0,
        owner: None,
    });
    true
}

fn url_in_quotes(rest: &str, quote: char) -> &str {
    &rest[..rest.find(quote).unwrap_or(rest.len())]
}
