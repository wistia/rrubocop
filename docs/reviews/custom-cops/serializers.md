# Custom serializer Rust port review

Reviewed commit `586bb4c`, read-only. No repository files changed.

## Scope and current exposure

Reviewed all five added serializer cops:

- `Serializers/AvoidCapitalizedKeys`
- `Serializers/AvoidKeyAndBlock`
- `Serializers/AvoidMethodAndBlock`
- `Serializers/PreferAttributeBlockOverAttributeAndMethod`
- `Serializers/PreferAttributeBlockOverAttributesAndMethods`

The first three are diagnostic-only (`supports_autocorrect == false` and no edits). I found no way for their diagnostics themselves to modify Ruby during either correction mode.

Both corrective policies are explicitly `AutoCorrect: disabled` in Wistia's `.rubocop.yml:283` and `:288`. Independently tested one hazardous input per policy under the real config with both `-a` and `-A`: each reports one offense and zero corrections. Thus the findings below are blocked today in the Wistia policy branch. They become reachable if the override is removed, set to `true`, or the rules are reused elsewhere. Both are currently registered as safe-correctable at `src/cop/ported/retained.rs:17` and `:18`, so ordinary `-a` is exposed after such a config change.

All four confirmed hazards below also reproduce with the original Ruby policies on pinned RuboCop 1.65.1. They are inherited unsafe algorithms, not newly introduced Rust-only behavior. Runtime observations use the installed ActiveModelSerializers0.10.16 `ActiveModel::Serializer::Field#value`, whose block path uses `serializer.instance_eval(&block)`.

## High: both corrective policies move method returns into a different control-flow context

Locations: `src/cop/ported/retained/serializers/correction.rs:7` and `:28`.

Before (singular; plural declaration behaves the same):

```ruby
class S
  attribute :name
  def name
    return "name"
  end
end
```

After:

```ruby
class S
  attribute :name do
    return "name"
  end
end
```

The original field evaluates to `"name"`; the corrected field raises `LocalJumpError: unexpected return`. Syntax remains valid, so `ruby -c` does not catch this. Removing an ordinary serializer method also needs caller analysis: other helper methods may invoke it; its disappearance can break those callers even when the moved body has no `return`.

Disposition: preserve disabled corrections. If implementing guarded corrections, reject non-local control-flow constructs and any method whose callers cannot be shown safe. Mark correction safety false as defense in depth.

## High: singular correction drops optional parameter bindings/defaults

Location: `src/cop/ported/retained/serializers/correction.rs:7`.

Before:

```ruby
class S
  attribute :name
  def name(prefix = "prefix")
    prefix + object.name
  end
end
```

After:

```ruby
class S
  attribute :name do
    prefix + object.name
  end
end
```

The original field evaluates to `"prefixobject"` for an object whose name is `"object"`; the corrected field raises `NameError` for `prefix`. Syntax still passes. The corrector copies only the body and removes the complete method, without checking its parameters.

Disposition: do not correct methods with parameters (including optional/keyword/rest/block params) unless those bindings are explicitly and correctly translated. Keeping this cop diagnostic-only is reasonable.

## High: plural correction destroys a conditional declaration's syntax

Location: `src/cop/ported/retained/serializers/correction.rs:36`.

Before:

```ruby
class S
  attributes :name if enabled?
  def name
    object.name
  end
end
```

After:

```ruby
class S
  if enabled?
  attribute :name do
    object.name
  end
end
```

The call-only removal leaves an unmatched `if`, making the entire file invalid. With other declaration wrappers, moving the block to the method's old position may also remove the declaration's condition rather than preserve it.

Disposition: require a standalone declaration; refuse modifier/conditional/wrapped calls. If preserving wrappers in a future implementation, use AST source ranges for the complete statement, not call text alone.

## High: plural textual method rewrite silently removes a serialized field

Location: `src/cop/ported/retained/serializers/correction.rs:28`.

Before:

```ruby
class S
  attributes :name
  def  name
    object.name
  end
end
```

After:

```ruby
class S
  def  name
    object.name
  end
end
```

Two spaces between `def` and the name are valid Ruby. `replacen("def name", ...)` finds no match, but the declaration is still removed. The field registry changes from `[:name]` to `[]`; serialization silently drops the key. Syntax remains valid and the next lint pass no longer has an attribute declaration to flag.

Disposition: replace keyword/name tokens via AST ranges, or refuse correction when the expected replacement cannot be proven. Never remove a declaration if method-to-block conversion did not occur.

## Positive observations and limits

- Same-scope matching excludes sibling and nested class methods, preserving the scope fix already added to Ruby and Rust.
- Commented argument lists are filtered correctly in the new corrective ports.
- Repeated singular declarations sharing one method produced two block declarations without dangling half-edits in the tested case.
- A simple heredoc method body is preserved correctly by Rust. The original Ruby single-attribute corrector drops that heredoc's body/terminator and emits invalid Ruby; Rust is safer here.
- Ordinary comments preceding a moved method survive, but can become detached documentation. This is a review concern, not proven runtime damage.
- The plural corrector's `do(prefix = "prefix")` output for a single optional positional default compiles and preserved the tested value. I did not count it as a defect; the confirmed parameter-loss finding applies to the singular corrector.
- No broad proof of correction safety is possible from clean corpus scans: the branch has zero offenses for these rules on the current application corpus, so adverse correction paths need adversarial fixtures.

## Reproduction artifacts

- `/tmp/rr-review-serializers-probe.py`: runs both CLIs with explicit correction enablement on eleven adversarial cases.
- `/tmp/rr-review-serializers-probes.json`: original sources, Ruby/Rust rewritten output, exit codes, and Ruby 3.4 syntax validation.
- `/tmp/rr-review-serializers-runtime.rb` and `.json`: actual AMS field evaluator before/after results.
- `/tmp/rr-review-serializers-disabled.json`: real Wistia config `-a`/`-A` exposure checks.

The valid-syntax checks were rerun with the absolute Ruby 3.4.9 executable because the Rust repository has no Ruby selection for the asdf shim.
