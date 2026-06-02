# Built-in Plugins Reference

This is the long-form companion to the table in [README](../README.md#-built-in-plugins). It explains what each detector looks for, what its default thresholds mean, and shows a minimal example that triggers it.

Conventions:

- Every plugin lives in [`cha-core/src/plugins/<name>.rs`](../cha-core/src/plugins). The defaults shown here come straight from `Default for <Analyzer>` in that file.
- Thresholds scale by the global `strictness` factor (`relaxed` = 2.0×, `default` = 1.0×, `strict` = 0.5×, or any custom float).
- Override per-plugin in `.cha.toml` under `[plugins.<name>]`. Override per-item in source via `// cha:set <field>=<value>` or `// cha:ignore <name>`.

---

## Bloaters

### `length`

Long methods, classes, and files.

Source: [`length.rs`](../cha-core/src/plugins/length.rs)

| Smell | Triggered when | Severity |
|-------|----------------|----------|
| `long_method` | A function exceeds `max_function_lines` (default 50). Promoted to Error if the function is **also** complex enough that `cyclomatic × cognitive ≥ complexity_factor_threshold` (default 10.0). Otherwise Warning. | Hint / Warning / Error |
| `large_class` | A class has more than `max_class_methods` methods (default 10) **or** spans more than `max_class_lines` lines (default 200). | Warning |
| `large_file` | A file is longer than `max_file_lines` (default 500). | Warning |

The `complexity_factor` gate exists so that a long-but-linear function (e.g. a 60-line config table builder) stays a Warning, while a long-and-twisty function (60 lines, complexity 12, cognitive 14 → factor 168) escalates to Error.

```toml
[plugins.length]
max_function_lines = 80
max_class_lines    = 300
```

### `complexity`

Cyclomatic complexity — the number of linearly independent paths through a function. Counts every branching keyword (`if`, `else if`, `while`, `for`, `case`, `&&`, `||`, `?`, `catch`).

Source: [`complexity.rs`](../cha-core/src/plugins/complexity.rs)

A function exceeding `warn_threshold` (default 10) emits a `high_complexity` Warning; exceeding `error_threshold` (default 20) escalates to Error.

```toml
[plugins.complexity]
warn_threshold  = 8
error_threshold = 15
```

### `cognitive_complexity`

Same finding shape as `complexity`, but the underlying metric weights nesting: an `if` inside an `if` inside a `for` costs more than three flat `if`s. A function whose cognitive score exceeds `threshold` (default 15) emits a Warning; exceeding `2 × threshold` escalates to Error.

Source: [`cognitive_complexity.rs`](../cha-core/src/plugins/cognitive_complexity.rs)

Use this alongside `complexity` rather than instead of it: `complexity` tracks how many test cases you'd need; `cognitive_complexity` tracks how hard the code is to read.

```toml
[plugins.cognitive_complexity]
threshold = 12
```

### `long_parameter_list`

A function with more than `max_params` parameters (default 5) emits a Warning.

Source: [`long_parameter_list.rs`](../cha-core/src/plugins/long_parameter_list.rs)

Suggested fix is *Introduce Parameter Object* or *Preserve Whole Object* — group related parameters into a struct so callers stop juggling positional arguments.

```toml
[plugins.long_parameter_list]
max_params = 7
```

### `primitive_obsession`

A function whose parameter list is at least `min_params` long (default 3) and whose primitive-typed share is at least `primitive_ratio` (default 0.8) emits a Hint.

Source: [`primitive_obsession.rs`](../cha-core/src/plugins/primitive_obsession.rs)

"Primitive" here means the built-in numeric / string / bool types of each language: `i32`, `f64`, `bool`, `String`, `&str`, `number`, `boolean`, `any`, etc. The smell is the friction-free slide from "this id is a `String`" to "this `String` is a user id, but so is that one, and I just passed them in the wrong order."

Suggested fix: *Replace Primitive with Object* — wrap the meaning in a typedef / newtype / value object.

```toml
[plugins.primitive_obsession]
min_params       = 4
primitive_ratio  = 0.9
```

### `data_clumps`

The same group of parameter **types** repeating in `min_occurrences` (default 3) different functions, with the group itself being at least `min_clump_size` (default 3) types long. Emits a Hint per clump.

Source: [`data_clumps.rs`](../cha-core/src/plugins/data_clumps.rs)

This is the structural cousin of `primitive_obsession`: instead of "lots of primitives in one function", it's "the same triple of `(String, String, i32)` shows up in five functions". Both point at the same fix — extract a struct.

```toml
[plugins.data_clumps]
min_clump_size  = 4
min_occurrences = 2
```

### `naming`

Source: [`naming.rs`](../cha-core/src/plugins/naming.rs)

| Smell | Triggered when | Severity |
|-------|----------------|----------|
| `naming_too_short` | A function or class name is shorter than `min_name_length` (default 2 chars). | Warning |
| `naming_too_long` | A function or class name is longer than `max_name_length` (default 50 chars). | Hint |
| `naming_convention` | A class name does not start with an uppercase letter (PascalCase violation). | Hint |

`naming_convention` is the only smell in Cha that ships with an auto-fix today: `cha fix` rewrites every reference to the class name in PascalCase via the `Plugin::try_fix` pathway. The rewrite is AST-aware — identifiers inside string literals and comments are left untouched.

The C language preset disables this plugin entirely; C uses `snake_case` by convention and reporting it as a "violation" produces nothing but noise.

```toml
[plugins.naming]
min_name_length = 3
max_name_length = 40
```

### `api_surface`

Source: [`api_surface.rs`](../cha-core/src/plugins/api_surface.rs)

A file emits the `large_api_surface` Warning when its exported (public) functions and classes exceed either an absolute count or a ratio of the file's total. Defaults are `max_exported_count = 20` and `max_exported_ratio = 0.8`. Files with fewer than 5 declarations are skipped — there's nothing meaningful to say about a 3-function file.

C and C++ source files use a separate, more permissive pair (`c_max_exported_count = 30`, `c_max_exported_ratio = 1.01`) because `.c` files routinely export every non-static function by design — the `.h` file is what controls visibility. C and C++ header files (`.h`, `.hpp`, `.hxx`, `.hh`, `.h++`) are skipped entirely when `skip_c_headers = true` (the default), since headers are public-API by definition.

```toml
[plugins.api_surface]
max_exported_count = 15
max_exported_ratio = 0.7
```

### `god_class`

Source: [`god_class.rs`](../cha-core/src/plugins/god_class.rs)

A class fires the `god_class` Warning only when **all three** signals trip together:

- **ATFD** (Access to Foreign Data) — distinct external classes / objects this class touches via any method — exceeds `max_external_refs` (default 5). Indicates the class reaches outside itself a lot.
- **WMC** (Weighted Method Count) — sum of cyclomatic complexity across all methods of the class — meets `min_wmc` (default 47). Indicates the class is doing a lot.
- **TCC** (Tight Class Cohesion) — fraction of method pairs that share at least one field — falls below `min_tcc` (default 0.33). Indicates the methods don't agree on what the class is *about*.

Defaults match the empirical thresholds derived from a 45-Java-project survey, and the AND-of-three combination keeps the false-positive rate down: a class that's just busy (high WMC) but cohesive doesn't trigger; a class that's just loosely cohesive but small doesn't trigger either. You need a class that's busy, scattered, and reaching outside.

Suggested fixes are *Extract Class* (split off responsibilities) and *Single Responsibility Principle* refactoring.

```toml
[plugins.god_class]
max_external_refs = 7
min_wmc           = 60
min_tcc           = 0.25
```

### `brain_method`

Source: [`brain_method.rs`](../cha-core/src/plugins/brain_method.rs)

The function-level cousin of `god_class`. A function fires the `brain_method` Warning when all three signals trip together:

- Line count meets `min_lines` (default 65).
- Cyclomatic complexity meets `min_complexity` (default 4).
- Distinct external references (variables / fields / functions from outside the function's own scope) meet `min_external_refs` (default 7).

A long-but-linear function won't trigger (low complexity); a short-but-twisty function won't either (low line count); a self-contained function with no external references stays clean even if it's long and complex. The AND combination targets specifically the function that is doing too much, in too many turns, while reaching into too many places.

Suggested fixes are *Extract Method* and *Move Method*.

```toml
[plugins.brain_method]
min_lines         = 80
min_complexity    = 6
min_external_refs = 10
```

---

## Couplers

### `coupling`

Source: [`coupling.rs`](../cha-core/src/plugins/coupling.rs)

A file with more than `max_imports` imports (default 15) emits the `high_coupling` Warning. If the count exceeds `2 × max_imports`, the severity escalates to Error.

Rust `mod` declarations are excluded from the count — they're module organisation, not external coupling.

```toml
[plugins.coupling]
max_imports = 12
```

### `hub_like_dependency`

Source: [`hub_like.rs`](../cha-core/src/plugins/hub_like.rs)

Same idea as `coupling` but with a higher bar (`max_imports = 20` by default) and a different framing: this isn't a file that's *too connected* (`coupling`), it's a file that has become a **hub** in the dependency graph — a single point that reaches into a large fraction of the system.

The two plugins overlap on purpose. `coupling` flags everyday "this file is doing too much"; `hub_like_dependency` flags the architectural smell of "this file is the bottleneck the rest of the codebase routes through". Suggested fix is splitting into smaller modules, or interposing a Facade.

```toml
[plugins.hub_like_dependency]
max_imports = 15
```

### `feature_envy`

Source: [`feature_envy.rs`](../cha-core/src/plugins/feature_envy.rs)

A function whose external references (≥ `min_refs`, default 3) are dominated by a *single* other object — that one object accounts for at least `external_ratio` (default 0.7) of the references — emits a Hint.

The classic example: `Order::shipping_total()` reads `customer.address`, `customer.country`, `customer.tax_zone`, `customer.discount_tier`. The method is sitting in `Order` but spends all its time pawing at `Customer`. The fix is to move the method to the class it's envious of (*Move Method*).

```toml
[plugins.feature_envy]
min_refs       = 4
external_ratio = 0.8
```

### `middle_man`

Source: [`middle_man.rs`](../cha-core/src/plugins/middle_man.rs)

A class with at least `min_methods` (default 3) methods, of which at least `delegation_ratio` (default 0.5 = 50%) do nothing but forward the call to another object, emits a Hint.

A class that's mostly delegation isn't pulling its weight — its callers could talk to the underlying object directly. Suggested fix is *Remove Middle Man*: have callers go straight to the delegate.

Note: a small amount of delegation is healthy (encapsulation, lifecycle management). The default 50% threshold is meant to catch the case where the class has degenerated into a pass-through shim, not to penalise normal facades.

```toml
[plugins.middle_man]
min_methods       = 4
delegation_ratio  = 0.6
```

### `message_chain`

Source: [`message_chain.rs`](../cha-core/src/plugins/message_chain.rs)

A function that contains a dotted access chain longer than `max_depth` (default 3) — e.g. `a.b.c.d.e` — emits a Warning. The chain is detected via tree-sitter (`field_expression` / `member_expression` / `attribute` / `selector_expression` depending on language), not text matching, so chains that span lines or contain method calls are still recognised.

The smell isn't the punctuation, it's the implied knowledge: the caller of `a.b().c().d().e()` knows the entire spine of intermediate types. Refactoring with *Hide Delegate* lets `a` expose `e` directly so callers stop depending on the shape of the middle.

```toml
[plugins.message_chain]
max_depth = 4
```

### `inappropriate_intimacy`

Source: [`inappropriate_intimacy.rs`](../cha-core/src/plugins/inappropriate_intimacy.rs)

When file `A` imports file `B` and file `B` also imports `A`, both files emit a Warning at the import site. This is the most direct symptom of two modules that belong together being split apart, or two unrelated modules that have grown entangled.

Detection resolves relative imports against the filesystem, trying common extensions (`.ts`, `.tsx`, `.rs`, `.py`, `.go`, `.cpp`, `.cc`, `.cxx`, `.c`, `.h`, `.hpp`, `.hxx`, `.js`, `.jsx`, `.mts`, `.cts`). Non-relative imports (npm packages, third-party crates) are ignored — the cycle has to be inside your project.

Suggested fixes are *Move Method* (push the responsibility to one side) or *Hide Delegate* (insert a third module that owns both).

### `layer_violation`

Source: [`layer_violation.rs`](../cha-core/src/plugins/layer_violation.rs)

Disabled by default — enable by configuring layer prefixes in `.cha.toml`:

```toml
[plugins.layer_violation]
enabled = true
layers  = "domain:0,service:1,controller:2,ui:3"
```

Each entry is `<path-prefix>:<level>`. A file whose path matches a prefix sits at that level. **A lower-level file that imports from a higher-level file emits an Error**: `domain` cannot import from `service`, `service` cannot import from `controller`, and so on. Imports going the other direction (higher → lower) are allowed.

Use this to pin clean architecture / hexagonal / onion style layering at lint time. Once configured, the rule fires in CI and prevents the slow drift where a domain entity quietly starts importing the database adapter.

### `async_callback_leak`

Source: [`async_callback_leak.rs`](../cha-core/src/plugins/async_callback_leak.rs)

A function whose **public signature** mentions a raw async-handle type — `JoinHandle`, `Future`, `Task`, `AbortHandle`, `Sender` / `Receiver`, `UnboundedSender` / `UnboundedReceiver`, `Promise`, `Awaitable`, `Coroutine`, `Queue`, `CancelFunc`, `WaitGroup`, `oneshot`, `mpsc` — either as a parameter type or as the return type, emits a Hint.

Spawner functions are exempt: a function whose name starts with `spawn`, `launch`, `start`, `run_async`, `fire_`, `dispatch_`, or `background_` legitimately exists to *produce* a handle and is skipped.

The smell is leaking concurrency primitives across module boundaries. Once a public API returns a `JoinHandle`, every caller has to know about your runtime and your lifecycle. Wrapping the handle in a domain type (something like `RenderJob` that internally holds the `JoinHandle`) lets callers cancel / await / inspect through your vocabulary instead of tokio's.

---

## OO Abusers

### `switch_statement`

Source: [`switch_statement.rs`](../cha-core/src/plugins/switch_statement.rs)

A function whose `switch` / `match` has more than `max_arms` arms (default 8) emits a Warning. Detection is AST-based via tree-sitter (`match_expression` for Rust, `switch_statement` for TypeScript / C / C++, `match_statement` for Python, `expression_switch_statement` and `type_switch_statement` for Go), so keywords inside strings or comments don't trigger.

The classic refactoring is *Replace Conditional with Polymorphism*: each arm becomes a method on a subclass / trait impl / variant, and the dispatcher disappears into a virtual call. Whether that's worth it depends on how often the arms change together — if you're adding a new arm every week, polymorphism pays off; if the set is stable, the switch is fine.

```toml
[plugins.switch_statement]
max_arms = 12
```

### `temporary_field`

Source: [`temporary_field.rs`](../cha-core/src/plugins/temporary_field.rs)

A class with at least `min_methods` (default 3) methods, and a field that is only read by at most `max_usage_ratio` (default 0.3 = 30%) of those methods, emits a Hint per offending field. Fields used in zero methods are skipped — those are dead, not temporary.

The smell is a field that exists "just in case" or only during one specific operation: a class that grows a `_intermediate_buffer` member used only by `compute()`, or a `_pending_request_id` used only by `cancel()`. Suggested fix is *Extract Class*: pull the field plus the methods that touch it into their own object.

```toml
[plugins.temporary_field]
min_methods      = 5
max_usage_ratio  = 0.25
```

### `refused_bequest`

Source: [`refused_bequest.rs`](../cha-core/src/plugins/refused_bequest.rs)

A subclass with at least `min_methods` (default 3), of which at least `min_override_ratio` (default 0.5 = 50%) override the parent, emits a Hint.

When a subclass overrides most of what it inherits, the inheritance relationship has stopped pulling its weight — the subclass isn't really an *is-a* of the parent, it's reusing the parent as a hidden member. Suggested fix is *Replace Inheritance with Delegation*: hold a parent instance as a field instead of inheriting, and the override-heavy subclass becomes an honest delegating wrapper. *Push Down Method* is the other option: if the parent has methods only one child uses, move them into that child.

```toml
[plugins.refused_bequest]
min_override_ratio = 0.6
min_methods        = 4
```

### `design_pattern`

Source: [`design_pattern.rs`](../cha-core/src/plugins/design_pattern.rs)

Six structural patterns are suggested, each as its own smell, all at Hint severity:

| Smell | Triggered when |
|-------|----------------|
| `strategy_pattern` | A function dispatches on a field whose name contains one of `type_field_keywords` (default `type`, `kind`, `role`, `action`, `mode`) with at least `strategy_min_arms` (default 4) arms. |
| `state_pattern` | Same shape, but the dispatch field name contains one of `state_field_keywords` (default `state`, `status`) with at least `state_min_arms` (default 3) arms. |
| `builder_pattern` | A function takes at least `builder_min_params` (default 7) parameters; or it takes at least `builder_alt_min_params` (default 5) parameters where at least `builder_alt_min_optional` (default 3) of them are optional. |
| `null_object_pattern` | The same field is null-checked across at least `null_object_min_count` (default 3) different functions. |
| `template_method_pattern` | A class has at least `template_min_methods` (default 4) methods, one of which calls at least `template_min_self_calls` (default 3) other methods on `self`. |
| `observer_pattern` | A class has fields whose types name `Listener` / `Observer` / `Callback` / `Handler`, and / or methods whose names contain `notify` / `emit` / `publish`. |

These are advisory — the patterns aren't always the right answer, especially for a `switch` over a small fixed set, or a 7-parameter constructor for a struct that genuinely has 7 logically-distinct fields. Read the suggestion as "this code shape often becomes cleaner under pattern X", not "this is wrong".

```toml
[plugins.design_pattern]
strategy_min_arms = 5
builder_min_params = 8

# Override keyword lists if your codebase uses different names
type_field_keywords  = ["type", "kind", "variant", "tag"]
state_field_keywords = ["state", "phase", "stage"]
```

---

## Change Preventers

The two plugins in this group don't read your code — they read your `git log`. Both run a single `git log` invocation per analysis (cached for the whole run) and answer questions about **how the project has actually been changing**, not how it's currently shaped.

### `shotgun_surgery`

Source: [`shotgun_surgery.rs`](../cha-core/src/plugins/shotgun_surgery.rs)

For each file, looks at the last `max_commits` commits (default 100) and counts how often it was modified together with each other file. If a partner file co-occurred at least `min_co_changes` (default 5) times, emit a Hint pointing at that pair. Each file reports up to its top-3 most-frequent co-change partners.

The smell: making one logical change always requires touching the same handful of files together. The fix is *Move Method* or *Move Field* — gather the scattered behaviour into a single class so the next change lands in one place.

False positives to expect: schema migrations, config files, build manifests, lock files. Add them to `exclude` in `.cha.toml`.

```toml
[plugins.shotgun_surgery]
min_co_changes = 8
max_commits    = 200
```

### `divergent_change`

Source: [`divergent_change.rs`](../cha-core/src/plugins/divergent_change.rs)

Same data source, opposite question: instead of "which files always change together", it asks "for how many distinct *reasons* has this single file been changed?"

A "reason" is the conventional-commit scope (`type(scope): subject`) of each commit, falling back to the first word of the subject if no scope is present. If the same file shows up under at least `min_distinct_reasons` (default 4) different scopes within the last `max_commits` commits (default 50), emit a Hint.

The smell: the file is doing too many different jobs and gets pulled into changes for unrelated concerns. Suggested fix is *Extract Class* — split the file along its scope boundaries.

This rule is heavily dependent on commit message hygiene. If your project doesn't use conventional commits, the fallback (first word of subject) approximates a topic but produces noisier results — consider raising the threshold.

```toml
[plugins.divergent_change]
min_distinct_reasons = 6
max_commits          = 100
```

---

## Dispensables

### `dead_code`

Source: [`dead_code.rs`](../cha-core/src/plugins/dead_code.rs)

A non-exported function or class that is never referenced — within the file, across the project's call graph, **and** not in `entry_points` — emits a Hint.

Three signals stack:

- **Same-file usage** — AST identifier scan, so substring matches inside string literals or comments don't count as "referenced".
- **Cross-file call graph** — the project-wide index from the parser; a function called from another file is alive even if it's not used in its own file.
- **Token-concat recovery** (C / C++ only) — for files with `#define ... ##` macros (X-macro dispatch tables), the analyzer scans the macro body for `prefix##X##suffix` slots and combines them with each call site's arguments to reconstruct probable expansion names like `_handleColorAttr`. These names are added to the in-file reference set so an X-macro doesn't drown the file in false positives.

`entry_points` lets you whitelist names that frameworks, runtimes, or build systems call but don't appear in your code: defaults include `main` / `new` / `default` / `drop` / `fmt` (Rust), `__init__` / `__new__` / `__call__` / `__enter__` / `__exit__` / `__del__` (Python), `init` (Go), `_start` (C), and `tokio_main` / `main_async`.

When `ctx.tree` isn't available the plugin falls back to a substring scan; you'll see this in unit tests, never in real CLI runs.

```toml
[plugins.dead_code]
entry_points = ["main", "wasm_main", "ffi_entry"]
```

### `duplicate_code`

Source: [`duplicate_code.rs`](../cha-core/src/plugins/duplicate_code.rs)

Two or more functions whose AST body hashes match, **and** which are each longer than 10 lines, emit a Warning per duplicate. The hash ignores identifier names and exact whitespace, so structurally identical functions are caught even when their variables are renamed.

The 10-line floor exists so trivial getters and one-liners that hash identically don't bury the report. Suggested fix: *Extract Method* / *Extract Function* / *Pull Up Method*.

This plugin has no configuration — duplication is duplication, and the line floor is a hard implementation detail rather than a tuning knob.

### `comments`

Source: [`comments.rs`](../cha-core/src/plugins/comments.rs)

A function with at least `min_lines` (default 10) lines, whose comment lines are more than `max_comment_ratio` (default 0.3 = 30%) of the body, emits a Hint.

The smell isn't comments per se — it's comments that are paying for missing structure. A 20-line function that needs 8 lines of comments to explain itself is usually a 20-line function that should be three smaller functions whose names carry the explanation.

```toml
[plugins.comments]
max_comment_ratio = 0.4
min_lines         = 15
```

### `lazy_class`

Source: [`lazy_class.rs`](../cha-core/src/plugins/lazy_class.rs)

A class with at most `max_methods` (default 1) methods **and** at most `max_lines` (default 10) lines emits a Hint. Interfaces / traits are exempt — those are intentionally minimal.

The default thresholds (≤ 1 method, ≤ 10 lines) are intentionally aggressive: they catch the textbook case of "I made a wrapper class for one helper function and never grew it". If your codebase has many small intentional value types, raise both limits.

```toml
[plugins.lazy_class]
max_methods = 2
max_lines   = 20
```

### `data_class`

Source: [`data_class.rs`](../cha-core/src/plugins/data_class.rs)

A class with at least `min_fields` (default 2), no behaviour methods (only field accessors / mutators / constructors / `Default`-style), and not an interface, emits a Hint.

The smell description is "anaemic domain model": the class holds state but has no opinion about it, so callers end up reading and writing fields directly. Suggested fix is *Move Method* — find the methods elsewhere in the codebase that operate on this class's data, and move them in.

A handful of true data-only types (DTOs at API boundaries, serialisation envelopes) genuinely belong as data classes. Suppress those with `// cha:ignore data_class`.

```toml
[plugins.data_class]
min_fields = 3
```

### `speculative_generality`

Source: [`speculative_generality.rs`](../cha-core/src/plugins/speculative_generality.rs)

An interface / trait with **0 or 1** implementations in the same file emits a Hint. No configuration — the rule is binary.

The smell: an abstraction that was added "in case we want to swap it out one day" but has only one implementation today. Until a second implementation exists, the interface is paying for optionality you aren't using. The fix is to inline the interface; if a second impl arrives later, the abstraction can be reintroduced then.

This is a same-file check by design — it deliberately won't flag a trait that's defined here but implemented in another module. Cross-file detection lives in the post-analysis pass `cross_layer_import` (not a Plugin trait detector).

### `todo_tracker`

Source: [`todo_tracker.rs`](../cha-core/src/plugins/todo_tracker.rs)

Each `TODO` / `FIXME` / `HACK` / `XXX` comment in the codebase emits a finding:

| Tag | Severity |
|-----|----------|
| `HACK` | Warning |
| `XXX` | Warning |
| `FIXME` | Hint |
| `TODO` | Hint |

Detection is word-bounded (`"TODOs"` doesn't trigger, neither does `methodo`). No configuration — the four tags and their severities are baked in.

---

## Security

### `hardcoded_secret`

Source: [`hardcoded_secret.rs`](../cha-core/src/plugins/hardcoded_secret.rs)

Each string literal is matched against a fixed list of secret-shaped regexes:

| Pattern | Matches |
|---------|---------|
| AWS Access Key | `AKIA[0-9A-Z]{16,}` |
| Private Key | `-----BEGIN (RSA \| EC \| DSA \| OPENSSH )?PRIVATE KEY-----` |
| GitHub Token | `gh[ps]_[A-Za-z0-9_]{36,}` |
| Slack Token | `xox[bpors]-[A-Za-z0-9-]{10,}` |
| JWT | `eyJ...eyJ...` (three base64-ish segments separated by dots) |
| Hex Secret | 32+ hex chars, whole literal |
| Long Base64-ish Secret | 40+ chars from base64 / urlsafe alphabet, whole literal |

Each match emits a Warning. Detection runs against `string_literal` AST nodes only, so a literal API-key-shaped substring inside a comment, identifier, or doc block doesn't trigger.

The "Hex Secret" and "Long Base64-ish Secret" patterns can produce false positives on long deterministic constants (test vectors, hash digests, embedded resource IDs). Suppress those individually with `// cha:ignore hardcoded_secret`.

No configuration today — patterns are hard-coded. Bring your own via a custom WASM plugin if you need org-specific rules.

### `unsafe_api`

Source: [`unsafe_api.rs`](../cha-core/src/plugins/unsafe_api.rs)

Per-language tree-sitter queries flag known-dangerous calls:

- **Rust**: `unsafe` block, `unsafe fn`
- **Python**: `eval`, `exec`, `os.system`, `subprocess.call`, `pickle.load` / `pickle.loads`
- **TypeScript**: `eval`, `innerHTML` assignment, React's `dangerouslySetInnerHTML` JSX attribute, `document.write`
- **C / C++**: `gets`, `sprintf`, `strcpy`, `strcat`, `system`
- **Go**: `exec.Command`, `template.HTML`

Each match emits a Warning with the dangerous name and a one-line reason. Because detection is AST-based, a string `"system(rm -rf /)"` in a log message doesn't trigger.

When `ctx.tree` isn't available, the plugin returns nothing — silence is preferred over the noise that grep-based detection produces inside strings and comments.

No configuration — the dangerous-API list is hard-coded.

### `error_handling`

Source: [`error_handling.rs`](../cha-core/src/plugins/error_handling.rs)

Two distinct smells share one detection pass:

- **`unwrap_abuse`** — A function with more than `max_unwraps_per_function` (default 3) `.unwrap()` or `.expect(...)` calls emits a Warning at every `.unwrap()` / `.expect()` site in that function. Detection is AST-based via `(call_expression (field_expression (field_identifier) @method))` matched against the names `unwrap` / `expect`.
- **`empty_catch`** — Any `catch` (TypeScript / JavaScript) or `except` (Python) block with no body, or whose body is just `pass` / a comment, emits a Warning. Rust is excluded because `match` arms with empty arms are usually intentional.

The threshold targets the file that has accumulated unwraps faster than its error model could absorb them. A single `.unwrap()` on a known-OK invariant is fine; ten in one function suggests the function should be returning `Result` itself.

```toml
[plugins.error_handling]
max_unwraps_per_function = 5
```
