use super::*;

pub(in crate::cop::ported) struct Finding {
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub edit: Option<(usize, usize, String)>,
    pub additional: Vec<(usize, usize, String)>,
    pub correctable: bool,
}
impl Finding {
    pub fn new(node: Node<'_>, message: impl Into<String>) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            message: message.into(),
            edit: None,
            additional: vec![],
            correctable: false,
        }
    }
    pub fn range(start: usize, end: usize, message: impl Into<String>) -> Self {
        Self {
            start,
            end,
            message: message.into(),
            edit: None,
            additional: vec![],
            correctable: false,
        }
    }
    pub fn replace_with<T: Into<String>>(
        self,
        c: &Context<'_>,
        node: Node<'_>,
        render: impl FnOnce() -> T,
    ) -> Self {
        self.edit_with(c, node.start_byte(), node.end_byte(), render)
    }

    pub fn edit_with<T: Into<String>>(
        mut self,
        c: &Context<'_>,
        start: usize,
        end: usize,
        render: impl FnOnce() -> T,
    ) -> Self {
        self.correctable = true;
        if c.render_edits && !c.config.autocorrect_disabled() {
            self.edit = Some((start, end, render().into()));
        }
        self
    }

    pub fn also_edit_with<T: Into<String>>(
        mut self,
        c: &Context<'_>,
        start: usize,
        end: usize,
        render: impl FnOnce() -> T,
    ) -> Self {
        if c.render_edits && !c.config.autocorrect_disabled() {
            self.additional.push((start, end, render().into()));
        }
        self
    }
}
