use super::*;

#[derive(Clone, Copy, Default)]
struct Summary {
    disabled: bool,
    scope: bool,
}
#[derive(Default)]
pub(super) struct Policies {
    classes: HashMap<Key, Summary>,
    parents: HashMap<Key, Key>,
}
impl Policies {
    pub(super) fn new(c: &Context<'_>, root: Node<'_>) -> Self {
        let mut result = Self::default();
        result.walk(c, root, None);
        result
    }
    pub(super) fn disabled(&self, n: Node<'_>) -> bool {
        self.classes.get(&key(n)).is_some_and(|s| s.disabled)
    }
    pub(super) fn scope(&self, n: Node<'_>) -> bool {
        self.classes.get(&key(n)).is_some_and(|s| s.scope)
    }
    pub(super) fn parent_disabled(&self, n: Node<'_>) -> Option<bool> {
        self.parents
            .get(&key(n))
            .map(|parent| self.classes[parent].disabled)
    }
    fn record_parent(&mut self, c: &Context<'_>, n: Node<'_>, policy: Option<Key>) {
        if let Some(policy) = policy
            && n.kind() == "method"
            && c.method(n) == "show?"
        {
            self.parents.insert(key(n), policy);
        }
    }
    fn walk(&mut self, c: &Context<'_>, n: Node<'_>, policy: Option<Key>) -> Summary {
        self.record_parent(c, n, policy);
        let nested = if is_policy(c, n) {
            Some(key(n))
        } else {
            policy
        };
        let mut summary = own_summary(c, n);
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            let child = self.walk(c, child, nested);
            summary.disabled |= child.disabled;
            summary.scope |= child.scope;
        }
        if is_policy(c, n) {
            self.classes.insert(key(n), summary);
        }
        summary
    }
}
fn is_policy(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "class" && field(n, "name").is_some_and(|name| c.text(name).ends_with("Policy"))
}
fn own_summary(c: &Context<'_>, n: Node<'_>) -> Summary {
    Summary {
        disabled: is_call(n) && c.method(n) == "disable_scoping!",
        scope: scope_class(c, n),
    }
}
fn scope_class(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "class"
        && matches!(
            superclass(c, n).as_deref(),
            Some("ApplicationPolicy::Scope" | "JwtPolicy::Scope")
        )
}

fn superclass<'a>(c: &Context<'a>, n: Node<'_>) -> Option<std::borrow::Cow<'a, str>> {
    field(n, "superclass")
        .and_then(|s| s.named_child(0))
        .and_then(|s| c.constant_name(s))
}
