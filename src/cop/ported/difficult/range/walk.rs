use super::*;
impl<'n> State<'_, '_, 'n> {
    pub(super) fn walk(&mut self, n: Node<'n>) {
        if n.kind() == "heredoc_body" {
            return;
        }
        if n.kind() == "heredoc_beginning" {
            for interpolation in self.heredocs.at(n) {
                self.walk(interpolation);
            }
            return;
        }
        self.before(n);
        if is_call(n) {
            self.call(n);
        } else {
            self.children(n);
        }
    }
    fn children(&mut self, n: Node<'n>) {
        for child in kids(n) {
            self.walk(child);
        }
        match n.kind() {
            "binary" => self.after_binary(n),
            "element_reference" => self.after_index(n),
            _ => {}
        }
    }

    fn before(&mut self, n: Node<'_>) {
        match n.kind() {
            "global_variable" => self.reference(n),
            "when" => self.valid = self.maximum(when_patterns(n)),
            "in_clause" => {
                self.valid = field(n, "pattern").and_then(|p| self.maximum(descendants(p)));
            }
            "binary" => {
                if let Some(count) = self.static_match(n) {
                    self.valid = Some(count);
                }
            }
            "identifier" => self.bare_call(n),
            _ => {}
        }
    }
    fn call(&mut self, n: Node<'n>) {
        let b = block(n);
        for child in kids(n).into_iter().filter(|ch| Some(*ch) != b) {
            self.walk(child);
        }
        if !safe(self.c, n) {
            self.after(self.c.method(n), field(n, "receiver"), first_argument(n));
        }
        if let Some(b) = b {
            self.walk(b);
        }
    }
    fn static_match(&self, n: Node<'_>) -> Option<usize> {
        (operator(self.c, n) == "=~")
            .then(|| field(n, "left").and_then(|n| self.count(n)))
            .flatten()
    }
    fn after_binary(&mut self, n: Node<'_>) {
        if self.static_match(n).is_none() {
            self.after(operator(self.c, n), field(n, "left"), field(n, "right"));
        }
    }
    fn after_index(&mut self, n: Node<'_>) {
        if assigned_index(n) {
            return;
        }
        let receiver = field(n, "object");
        self.after(
            "[]",
            receiver,
            kids(n).into_iter().find(|n| Some(*n) != receiver),
        );
    }
    fn after(&mut self, method: &str, receiver: Option<Node<'_>>, argument: Option<Node<'_>>) {
        if !restricted(method) {
            return;
        }
        self.valid = None;
        if let Some(r) = argument
            .filter(|r| r.kind() == "regex" && argument_method(method))
            .or_else(|| receiver.filter(|r| r.kind() == "regex"))
        {
            self.valid = self.count(r);
        }
    }
    fn bare_call(&mut self, n: Node<'_>) {
        let name = self.c.text(n);
        if restricted(name) && !identifier_slot(n) && !local_identifier(self.c, n) {
            self.after(name, None, None);
        }
    }
}
fn when_patterns(n: Node<'_>) -> Vec<Node<'_>> {
    kids(n)
        .into_iter()
        .filter(|n| n.kind() == "pattern")
        .flat_map(kids)
        .collect()
}
fn first_argument(n: Node<'_>) -> Option<Node<'_>> {
    args(n).into_iter().find(|n| n.kind() != "comment")
}
fn identifier_slot(n: Node<'_>) -> bool {
    n.parent().is_some_and(|p| {
        field(p, "method") == Some(n) || field(p, "name") == Some(n) || field(p, "left") == Some(n)
    })
}

fn assigned_index(mut n: Node<'_>) -> bool {
    while let Some(parent) = n.parent() {
        match parent.kind() {
            "left_assignment_list" | "destructured_left_assignment" => n = parent,
            "assignment" => return field(parent, "left") == Some(n),
            "for" => return field(parent, "pattern") == Some(n),
            _ => return false,
        }
    }
    false
}
