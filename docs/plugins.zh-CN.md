# 内置插件参考

[README](../README.zh-CN.md#-内置插件) 那张表格的详细版。每个检测器到底在查什么、阈值的含义、什么样的代码会触发它，都在这里。

几点约定：

- 每个插件的源码都在 [`cha-core/src/plugins/<name>.rs`](../cha-core/src/plugins)，下面写的默认值就是各文件里 `Default for <Analyzer>` 写死的那些数。
- 阈值都会再乘以全局 `strictness`（`relaxed` 2.0×、`default` 1.0×、`strict` 0.5×，也可以写任意小数）。
- 想改某个插件，在 `.cha.toml` 的 `[plugins.<name>]` 节里覆盖；只想放过单个函数 / 类，在源码里写 `// cha:set <字段>=<值>` 或 `// cha:ignore <名字>`。

---

## Bloaters

### `length`

抓过长的函数、过大的类、过大的文件。

源码：[`length.rs`](../cha-core/src/plugins/length.rs)

| smell | 触发条件 | 严重度 |
|-------|---------|-------|
| `long_method` | 函数行数超过 `max_function_lines`（默认 50）。如果这个函数本身又复杂——`cyclomatic × cognitive ≥ complexity_factor_threshold`（默认 10.0）——直接升 Error；否则停在 Warning。 | Hint / Warning / Error |
| `large_class` | 类里方法数超过 `max_class_methods`（默认 10），或者类的总行数超过 `max_class_lines`（默认 200）。 | Warning |
| `large_file` | 文件超过 `max_file_lines`（默认 500）行。 | Warning |

`complexity_factor` 这道闸门是为了不冤枉那些"长但顺"的函数：一段 60 行的 lookup 表构造，复杂度低，那就停在 Warning；同样 60 行但绕来绕去的（complexity 12 × cognitive 14 = 168），就升级到 Error。

```toml
[plugins.length]
max_function_lines = 80
max_class_lines    = 300
```

### `complexity`

圈复杂度，也就是函数里能走出多少条互相独立的路径。每碰到一个分支关键字（`if`、`else if`、`while`、`for`、`case`、`&&`、`||`、`?`、`catch`）就 +1。

源码：[`complexity.rs`](../cha-core/src/plugins/complexity.rs)

超过 `warn_threshold`（默认 10）报 `high_complexity` Warning；超过 `error_threshold`（默认 20）直接 Error。

```toml
[plugins.complexity]
warn_threshold  = 8
error_threshold = 15
```

### `cognitive_complexity`

输出形式跟 `complexity` 一样，但算分方式不同——它会对嵌套加重惩罚。三个并排的 `if` 比一个 `if` 套 `if` 再套 `for` 便宜得多。

源码：[`cognitive_complexity.rs`](../cha-core/src/plugins/cognitive_complexity.rs)

得分超过 `threshold`（默认 15）报 Warning；超过两倍阈值升 Error。

它跟 `complexity` 是互补的，不是二选一：`complexity` 关心的是"要写多少测试才能覆盖"，`cognitive_complexity` 关心的是"读着累不累"。同一个函数两边都飘红，基本就是该重构了。

```toml
[plugins.cognitive_complexity]
threshold = 12
```

### `long_parameter_list`

参数超过 `max_params` 个（默认 5）报 Warning。

源码：[`long_parameter_list.rs`](../cha-core/src/plugins/long_parameter_list.rs)

推荐的修法是把相关参数打包成一个结构体（*Introduce Parameter Object* / *Preserve Whole Object*），调用方就不用记参数顺序了——参数一多最容易出的 bug 就是把两个 `String` 传反。

```toml
[plugins.long_parameter_list]
max_params = 7
```

### `primitive_obsession`

参数至少 `min_params` 个（默认 3）、且其中基本类型占比超过 `primitive_ratio`（默认 0.8）时报 Hint。

源码：[`primitive_obsession.rs`](../cha-core/src/plugins/primitive_obsession.rs)

"基本类型"是各语言自带的数字 / 字符串 / 布尔之类——`i32`、`f64`、`bool`、`String`、`&str`、`number`、`boolean`、`any`。这条 smell 抓的就是那种"用户 id 是 `String`、订单 id 也是 `String`，传参时两个一掉换我没察觉"的失控滑坡。

修法是 *Replace Primitive with Object*：把语义裹进一个 newtype 或值对象里，类型系统就能替你拦下传错。

```toml
[plugins.primitive_obsession]
min_params       = 4
primitive_ratio  = 0.9
```

### `data_clumps`

同一组参数类型在至少 `min_occurrences`（默认 3）个不同函数里反复出现，并且这组类型本身长度至少 `min_clump_size`（默认 3）。每抓到一组报一个 Hint。

源码：[`data_clumps.rs`](../cha-core/src/plugins/data_clumps.rs)

跟 `primitive_obsession` 是同一类问题的两个角度：那条是"一个函数里塞了一堆 primitive"，这条是"`(String, String, i32)` 这三联体在五个函数里都出现了"。修法也是同一个——抽出一个结构体。

```toml
[plugins.data_clumps]
min_clump_size  = 4
min_occurrences = 2
```

### `naming`

源码：[`naming.rs`](../cha-core/src/plugins/naming.rs)

| smell | 触发条件 | 严重度 |
|-------|---------|-------|
| `naming_too_short` | 函数 / 类名短于 `min_name_length`（默认 2 个字符）。 | Warning |
| `naming_too_long` | 函数 / 类名长于 `max_name_length`（默认 50 个字符）。 | Hint |
| `naming_convention` | 类名首字母不是大写（违反 PascalCase）。 | Hint |

`naming_convention` 是目前 Cha 里唯一带自动修复的 smell —— `cha fix` 会通过 `Plugin::try_fix` 把代码里所有引用都改成 PascalCase。改名走的是 AST 路径，字符串字面量和注释里的同名字符串不会被误伤。

C 语言预设里这个插件是关掉的：C 用 `snake_case` 是惯例，报"违反"只会满屏噪音。

```toml
[plugins.naming]
min_name_length = 3
max_name_length = 40
```

### `api_surface`

源码：[`api_surface.rs`](../cha-core/src/plugins/api_surface.rs)

一个文件里导出的（public）函数和类，要么数量超过绝对值（默认 `max_exported_count = 20`），要么占比超过总声明数的 `max_exported_ratio`（默认 0.8），就报一个 `large_api_surface` Warning。一个文件里如果一共还没 5 个声明，直接跳过——3 个函数的文件谈"暴露过多"没意义。

C / C++ 源文件走一套更宽松的阈值（`c_max_exported_count = 30`、`c_max_exported_ratio = 1.01`），因为 `.c` 里 non-static 函数本来就是默认全导出，是 `.h` 头文件在控制可见性，对 `.c` 算"导出占比"几乎一定 100%。`.h` / `.hpp` / `.hxx` / `.hh` / `.h++` 这些头文件本身就是公开 API，开了 `skip_c_headers`（默认 `true`）会整个跳过。

```toml
[plugins.api_surface]
max_exported_count = 15
max_exported_ratio = 0.7
```

### `god_class`

源码：[`god_class.rs`](../cha-core/src/plugins/god_class.rs)

一个类要触发 `god_class` Warning，必须**三个信号同时**满足：

- **ATFD**（Access to Foreign Data）：这个类各个方法访问到的"外部类 / 对象"种类数，超过 `max_external_refs`（默认 5）。说明它伸手伸太远。
- **WMC**（Weighted Method Count）：所有方法的圈复杂度之和，达到 `min_wmc`（默认 47）。说明它干得太多。
- **TCC**（Tight Class Cohesion）：方法两两之间共享至少一个字段的比例，低于 `min_tcc`（默认 0.33）。说明这些方法对"这个类要干嘛"看法不一致。

默认值是经验阈值（来自一份 45 个 Java 项目的统计）。三个信号取交集是为了压假阳性：一个类只是忙（WMC 高）但还内聚，不报；一个类内聚低但本身很小，也不报。**忙、散、还往外伸**三件事必须凑齐。

修法是 *Extract Class*（拆掉一部分职责）或者直接按单一职责原则重构。

```toml
[plugins.god_class]
max_external_refs = 7
min_wmc           = 60
min_tcc           = 0.25
```

### `brain_method`

源码：[`brain_method.rs`](../cha-core/src/plugins/brain_method.rs)

`god_class` 的函数级对应。一个函数要报 `brain_method` Warning，三个信号必须同时满足：

- 行数达到 `min_lines`（默认 65）。
- 圈复杂度达到 `min_complexity`（默认 4）。
- 外部引用（来自函数自身作用域之外的变量 / 字段 / 函数）的种类数达到 `min_external_refs`（默认 7）。

只长不绕的函数（低复杂度）不会报；只绕不长的函数（行数不够）也不会报；行数和复杂度都飘红但全是自包含计算，外部引用为零，也不会报。三个信号取交集，刚好夹住那种"做太多事、绕太多弯、还伸太多手"的函数。

修法是 *Extract Method*（拆函数）和 *Move Method*（搬到该归属的类去）。

```toml
[plugins.brain_method]
min_lines         = 80
min_complexity    = 6
min_external_refs = 10
```

---

## Couplers

### `coupling`

源码：[`coupling.rs`](../cha-core/src/plugins/coupling.rs)

文件 import 数超过 `max_imports`（默认 15）报 `high_coupling` Warning；超过 `2 × max_imports` 升级到 Error。

Rust 的 `mod` 声明不算在内——那是模块组织，不是对外耦合。

```toml
[plugins.coupling]
max_imports = 12
```

### `hub_like_dependency`

源码：[`hub_like.rs`](../cha-core/src/plugins/hub_like.rs)

跟 `coupling` 是同一类信号，但门槛更高（默认 `max_imports = 20`），关注角度也不一样：这条不是说"这个文件做太多事"（那是 `coupling`），而是说这个文件已经成了依赖图里的**枢纽节点**——一个伸进系统大半的中转站。

两条插件刻意有重合。`coupling` 抓的是日常意义上"这个文件管太多"，`hub_like_dependency` 抓的是架构意义上"整个项目都从这一个文件转一道"。修法是拆模块，或者插一层 Facade 把扇出收拢。

```toml
[plugins.hub_like_dependency]
max_imports = 15
```

### `feature_envy`

源码：[`feature_envy.rs`](../cha-core/src/plugins/feature_envy.rs)

一个函数的外部引用至少 `min_refs` 个（默认 3），其中**单一对象**就占了至少 `external_ratio`（默认 0.7）的份额，报 Hint。

经典例子：`Order::shipping_total()` 一上来读 `customer.address`、`customer.country`、`customer.tax_zone`、`customer.discount_tier`。这个方法挂在 `Order` 上，但全程都在扒 `Customer`。修法是把方法搬到它惦记的那个类去（*Move Method*）。

```toml
[plugins.feature_envy]
min_refs       = 4
external_ratio = 0.8
```

### `middle_man`

源码：[`middle_man.rs`](../cha-core/src/plugins/middle_man.rs)

一个类至少有 `min_methods` 个方法（默认 3），其中至少 `delegation_ratio`（默认 0.5）的方法只是把调用转给别的对象，报 Hint。

一个类如果绝大部分方法都是转发，它本身没在干活——调用方完全可以直接找下游对象。修法是 *Remove Middle Man*：让调用方绕过去。

注意：少量委托是健康的（封装、生命周期管理）。50% 这个默认值要抓的是那种"已经退化成透传薄壳"的类，不是要把正常的 facade 一起拍掉。

```toml
[plugins.middle_man]
min_methods       = 4
delegation_ratio  = 0.6
```

### `message_chain`

源码：[`message_chain.rs`](../cha-core/src/plugins/message_chain.rs)

函数里出现长于 `max_depth`（默认 3）的点访问链——比如 `a.b.c.d.e`——报 Warning。链路通过 tree-sitter 识别（按语言对应 `field_expression` / `member_expression` / `attribute` / `selector_expression`），不是文本匹配，所以跨行或夹了方法调用的链照样能抓出来。

要抓的不是那串点号，而是它隐含的耦合：`a.b().c().d().e()` 的调用方知道整条中间链路上每一层的类型。用 *Hide Delegate* 让 `a` 直接对外暴露 `e`，调用方就不用再依赖中间这堆类型形状。

```toml
[plugins.message_chain]
max_depth = 4
```

### `inappropriate_intimacy`

源码：[`inappropriate_intimacy.rs`](../cha-core/src/plugins/inappropriate_intimacy.rs)

文件 `A` 引入 `B`，同时 `B` 也引入 `A`，两边都在 import 那一行报 Warning。这是"本来该合在一起的两个模块被拆开了"或者"两个不相关的模块互相缠在一起"最直接的征兆。

检测时把相对路径解析到磁盘，依次试常见扩展名（`.ts`、`.tsx`、`.rs`、`.py`、`.go`、`.cpp`、`.cc`、`.cxx`、`.c`、`.h`、`.hpp`、`.hxx`、`.js`、`.jsx`、`.mts`、`.cts`）。非相对路径（npm 包、第三方 crate）一律忽略——循环必须发生在你自己项目里。

修法是 *Move Method*（把责任推到一边去）或 *Hide Delegate*（拉出一个第三方模块同时持有两边）。

### `layer_violation`

源码：[`layer_violation.rs`](../cha-core/src/plugins/layer_violation.rs)

默认是关的，要先在 `.cha.toml` 里配层级：

```toml
[plugins.layer_violation]
enabled = true
layers  = "domain:0,service:1,controller:2,ui:3"
```

每一项是 `<路径前缀>:<层级>`。文件路径能匹到哪个前缀就属于哪一层。**底层文件 import 高层文件，直接 Error**：`domain` 不能 import `service`，`service` 不能 import `controller`，以此类推。反向（高层 import 底层）是允许的。

用它在 lint 阶段把整洁架构 / hexagonal / onion 那种分层规则钉死。配好之后，CI 会拦下那种"domain 实体悄悄开始 import 数据库适配器"的慢性漂移。

### `async_callback_leak`

源码：[`async_callback_leak.rs`](../cha-core/src/plugins/async_callback_leak.rs)

一个函数的**对外签名**里出现裸的异步句柄类型——`JoinHandle`、`Future`、`Task`、`AbortHandle`、`Sender` / `Receiver`、`UnboundedSender` / `UnboundedReceiver`、`Promise`、`Awaitable`、`Coroutine`、`Queue`、`CancelFunc`、`WaitGroup`、`oneshot`、`mpsc`——无论是作为参数类型还是返回类型，都报 Hint。

启动器函数会被豁免：函数名以 `spawn`、`launch`、`start`、`run_async`、`fire_`、`dispatch_`、`background_` 开头的，存在的意义本来就是产生句柄，跳过。

要抓的是把并发原语漏过模块边界。一旦你的公开 API 返回 `JoinHandle`，每一个调用方都得知道你用的是哪套 runtime、怎么管生命周期。把句柄包进领域类型（比如 `RenderJob` 内部持有 `JoinHandle`），调用方就用你自己的词汇 cancel / await / 查询，不用学 tokio 那一套。

---

## OO Abusers

### `switch_statement`

源码：[`switch_statement.rs`](../cha-core/src/plugins/switch_statement.rs)

函数里的 `switch` / `match` 分支数超过 `max_arms`（默认 8）报 Warning。判定走 tree-sitter（Rust 的 `match_expression`、TypeScript / C / C++ 的 `switch_statement`、Python 的 `match_statement`、Go 的 `expression_switch_statement` / `type_switch_statement`），不是字符串匹配，所以注释和字符串里出现的关键字不会误报。

经典修法是 *Replace Conditional with Polymorphism*：每个分支变成子类 / trait 实现 / 枚举变体的一个方法，调度本身消化进多态调用。值不值得做要看分支组改的频率：如果几乎每周都加一个新分支，多态划算；如果分支集合稳定，留着 switch 反而清楚。

```toml
[plugins.switch_statement]
max_arms = 12
```

### `temporary_field`

源码：[`temporary_field.rs`](../cha-core/src/plugins/temporary_field.rs)

一个类至少有 `min_methods` 个方法（默认 3），其中某个字段只被不超过 `max_usage_ratio`（默认 0.3，即 30%）的方法用到，每个这样的字段报一个 Hint。零使用的字段不算——那是死代码不是临时字段。

要抓的是那种"以防万一加一个"或者"只在特定场景活一会儿"的字段：一个 `_intermediate_buffer` 只被 `compute()` 用、一个 `_pending_request_id` 只被 `cancel()` 用。修法是 *Extract Class*：把这个字段和真正用它的那几个方法一起拆成新对象。

```toml
[plugins.temporary_field]
min_methods      = 5
max_usage_ratio  = 0.25
```

### `refused_bequest`

源码：[`refused_bequest.rs`](../cha-core/src/plugins/refused_bequest.rs)

子类至少有 `min_methods` 个方法（默认 3），其中至少 `min_override_ratio`（默认 0.5）覆盖了父类，报 Hint。

子类把继承下来的东西改写过半，继承关系就名存实亡了——这子类已经不是父类的"是一个"，只是把父类当藏起来的成员在用。修法是 *Replace Inheritance with Delegation*：把父类换成一个字段持有，"override 大半"的子类就老老实实变成一层包装。或者反过来用 *Push Down Method*：如果父类的某些方法只有一个子类在用，把它们直接搬下去。

```toml
[plugins.refused_bequest]
min_override_ratio = 0.6
min_methods        = 4
```

### `design_pattern`

源码：[`design_pattern.rs`](../cha-core/src/plugins/design_pattern.rs)

提示六种结构性模式，各自一个 smell，全部 Hint 级别：

| smell | 触发条件 |
|-------|---------|
| `strategy_pattern` | 函数在某个字段上 dispatch，字段名包含 `type_field_keywords` 之一（默认 `type`、`kind`、`role`、`action`、`mode`），且分支数至少 `strategy_min_arms`（默认 4）。 |
| `state_pattern` | 同样的形状，但 dispatch 的字段名包含 `state_field_keywords` 之一（默认 `state`、`status`），分支数至少 `state_min_arms`（默认 3）。 |
| `builder_pattern` | 函数参数数至少 `builder_min_params`（默认 7）；或者参数数至少 `builder_alt_min_params`（默认 5），且其中可选参数至少 `builder_alt_min_optional`（默认 3）。 |
| `null_object_pattern` | 同一个字段在至少 `null_object_min_count`（默认 3）个不同函数里都被做了 null check。 |
| `template_method_pattern` | 一个类至少有 `template_min_methods`（默认 4）个方法，其中某个方法在 `self` 上调用了至少 `template_min_self_calls`（默认 3）个其他方法。 |
| `observer_pattern` | 类有名字带 `Listener` / `Observer` / `Callback` / `Handler` 的字段，并 / 或有名字带 `notify` / `emit` / `publish` 的方法。 |

这些都是建议性的——模式不一定永远是对的答案，比如分支固定的小 switch、或者 7 个参数确实是 7 个逻辑独立字段的构造函数。建议只是"这个形状常见地能在模式 X 下变干净"，不是"这是错的"。

```toml
[plugins.design_pattern]
strategy_min_arms = 5
builder_min_params = 8

# 你项目里用的字段命名不一样的话覆盖这些列表
type_field_keywords  = ["type", "kind", "variant", "tag"]
state_field_keywords = ["state", "phase", "stage"]
```

---

## Change Preventers

这一组的两条插件不读代码，读的是 `git log`。每次分析跑一次 `git log` 然后整轮缓存，回答的是"这个项目实际上是**怎么被改的**"，不是它现在长什么样。

### `shotgun_surgery`

源码：[`shotgun_surgery.rs`](../cha-core/src/plugins/shotgun_surgery.rs)

对每个文件，看过去 `max_commits` 个 commit（默认 100），统计它跟其他每个文件一起被改的次数。某个搭档文件共出现至少 `min_co_changes` 次（默认 5），就为这一对报一个 Hint。每个文件最多报最常一起出现的前 3 个搭档。

要抓的形态：每次做一个逻辑变更都得同时改一组固定的文件。修法是 *Move Method* 或 *Move Field*——把散在各处的行为聚到一个类里，下一次相同的变更只需要改一处。

容易假阳性的几类：迁移脚本、配置文件、构建清单、锁文件。这些放进 `.cha.toml` 的 `exclude` 里。

```toml
[plugins.shotgun_surgery]
min_co_changes = 8
max_commits    = 200
```

### `divergent_change`

源码：[`divergent_change.rs`](../cha-core/src/plugins/divergent_change.rs)

同一份数据，反过来问：不是"哪些文件总一起改"，而是"这一个文件**因为多少种不同原因**被改过"。

"原因" 取的是 conventional commit 的 scope（`type(scope): subject` 里的 scope），如果没有 scope 就退而取主题第一个词。同一个文件在过去 `max_commits` 次 commit（默认 50）里跨过至少 `min_distinct_reasons`（默认 4）种 scope，就报一个 Hint。

要抓的形态：这个文件干的事太杂，各种不相关的需求都会扯到它。修法是 *Extract Class*——按 scope 边界把文件切开。

这条规则非常依赖 commit message 的卫生。项目没用 conventional commits 的话，fallback（取主题第一个词）只是近似分组，结果会更糙——可以把阈值调高。

```toml
[plugins.divergent_change]
min_distinct_reasons = 6
max_commits          = 100
```

---

## Dispensables

### `dead_code`

源码：[`dead_code.rs`](../cha-core/src/plugins/dead_code.rs)

一个非导出的函数或类，文件内、全项目调用图里都没人引用，**而且**也不在 `entry_points` 名单里——报 Hint。

三层信号叠加：

- **同文件使用** —— 走 AST 标识符扫描。字符串字面量、注释里出现的同名子串不算"引用"。
- **跨文件调用图** —— 来自 parser 的全项目索引；本文件不用、但别的文件调用过的函数仍然算活的。
- **Token-concat 还原**（仅 C / C++）—— 文件里有 `#define ... ##` 这种宏（X-macro 派发表）时，分析器会扫宏体里的 `prefix##X##suffix` 槽位，再扫每一处调用点的实参，反推出可能的展开名字（比如 `_handleColorAttr`）。这些名字会被加进文件的引用集合，避免一个 X-macro 把整个文件的真函数都按死代码报掉。

`entry_points` 是给框架 / runtime / 构建系统调用、但你代码里看不到的函数留的白名单：默认包含 Rust 的 `main` / `new` / `default` / `drop` / `fmt`，Python 的 `__init__` / `__new__` / `__call__` / `__enter__` / `__exit__` / `__del__`，Go 的 `init`，C 的 `_start`，tokio 的 `tokio_main` / `main_async`。

如果 `ctx.tree` 不可用，插件会退回到子串扫描——这只会在 unit test 场景遇到，CLI 实际跑不会触发。

```toml
[plugins.dead_code]
entry_points = ["main", "wasm_main", "ffi_entry"]
```

### `duplicate_code`

源码：[`duplicate_code.rs`](../cha-core/src/plugins/duplicate_code.rs)

两个或更多函数的 AST 结构哈希一致，**且**每个都超过 10 行，每一份重复都报一个 Warning。哈希计算忽略变量名和具体空白，所以结构一样、变量重命名过的"双胞胎"也能抓出来。

10 行下限是为了不让 trivial 的 getter 和一行函数刷屏（它们经常哈希相同）。修法：*Extract Method* / *Extract Function* / *Pull Up Method*。

这条插件没配置项——重复就是重复，10 行下限是写死的实现细节而非旋钮。

### `comments`

源码：[`comments.rs`](../cha-core/src/plugins/comments.rs)

函数至少 `min_lines` 行（默认 10），且其中注释行占比超过 `max_comment_ratio`（默认 0.3，即 30%），报 Hint。

要抓的不是"注释多本身"，而是"用注释来填补结构上的缺失"。一个 20 行函数要写 8 行注释才能解释清楚，通常意味着它应该被拆成三个更小的函数，让函数名替注释发声。

```toml
[plugins.comments]
max_comment_ratio = 0.4
min_lines         = 15
```

### `lazy_class`

源码：[`lazy_class.rs`](../cha-core/src/plugins/lazy_class.rs)

类的方法数不超过 `max_methods`（默认 1）**且**总行数不超过 `max_lines`（默认 10）报 Hint。Interface / trait 不算——那本来就是刻意保持很小的。

默认值（≤ 1 个方法、≤ 10 行）故意打得很狠，要抓的是教科书式的"为一个 helper 包了个壳，之后再也没长大"。如果你项目里本来就有大量小但有意为之的值类型，把两个上限调高。

```toml
[plugins.lazy_class]
max_methods = 2
max_lines   = 20
```

### `data_class`

源码：[`data_class.rs`](../cha-core/src/plugins/data_class.rs)

类至少有 `min_fields`（默认 2）个字段、没有任何行为方法（只有字段访问器 / 修改器 / 构造器 / `Default` 之类）、并且不是 interface——报 Hint。

要抓的形态叫"贫血领域模型"：类只是个状态容器，对自己持有的数据没观点，调用方只好直接读写它的字段。修法是 *Move Method*——找到代码库其他地方那些专门处理这个类数据的函数，搬进来。

确实该是纯数据的类型（API 边界的 DTO、序列化封装）就老老实实是 data class。这种情况用 `// cha:ignore data_class` 压掉。

```toml
[plugins.data_class]
min_fields = 3
```

### `speculative_generality`

源码：[`speculative_generality.rs`](../cha-core/src/plugins/speculative_generality.rs)

interface / trait 在同一个文件里有 **0 个或 1 个**实现，报 Hint。没配置项——规则就是二选一。

要抓的形态：一个抽象当初为了"以后可能要换实现"加上去，结果到现在只有一个实现。在第二个实现出现之前，这个抽象等于在为你不用的可选性付维护税。修法是把 interface 内联掉；以后真有第二个实现需要时再抽出来不晚。

设计上这条只看本文件。同文件定义、跨模块实现的 trait 不会触发——跨文件检测交给 post-analysis pass `cross_layer_import`（那不是 Plugin trait 体系下的检测器）。

### `todo_tracker`

源码：[`todo_tracker.rs`](../cha-core/src/plugins/todo_tracker.rs)

代码里每一条 `TODO` / `FIXME` / `HACK` / `XXX` 注释都报一个 finding：

| 标签 | 严重度 |
|------|-------|
| `HACK` | Warning |
| `XXX` | Warning |
| `FIXME` | Hint |
| `TODO` | Hint |

匹配是按词边界的（`"TODOs"` 不会触发，`methodo` 也不会）。没配置项——四种标签和各自的严重度都写死了。

---

## Security

### `hardcoded_secret`

源码：[`hardcoded_secret.rs`](../cha-core/src/plugins/hardcoded_secret.rs)

每个字符串字面量会跟一组固定的"密钥形状"正则匹配：

| 模式 | 匹配 |
|------|------|
| AWS Access Key | `AKIA[0-9A-Z]{16,}` |
| Private Key | `-----BEGIN (RSA \| EC \| DSA \| OPENSSH )?PRIVATE KEY-----` |
| GitHub Token | `gh[ps]_[A-Za-z0-9_]{36,}` |
| Slack Token | `xox[bpors]-[A-Za-z0-9-]{10,}` |
| JWT | `eyJ...eyJ...`（点分隔三段 base64 风格） |
| Hex Secret | 整段字面量是 32+ 位 hex |
| Long Base64-ish Secret | 整段字面量是 40+ 位 base64 / urlsafe 字符 |

每命中一条报一个 Warning。匹配只在 `string_literal` AST 节点上跑，注释、标识符、doc 块里出现的同样字符串不会触发。

"Hex Secret" 和 "Long Base64-ish Secret" 这两条对长确定性常量会假阳性（测试向量、哈希摘要、嵌入资源 ID）。这种逐行用 `// cha:ignore hardcoded_secret` 压掉。

目前没配置项——模式是写死的。要按你团队的规则做扩展，写个 WASM 插件挂上去。

### `unsafe_api`

源码：[`unsafe_api.rs`](../cha-core/src/plugins/unsafe_api.rs)

按语言用 tree-sitter query 匹配已知危险调用：

- **Rust**：`unsafe` 块、`unsafe fn`
- **Python**：`eval`、`exec`、`os.system`、`subprocess.call`、`pickle.load` / `pickle.loads`
- **TypeScript**：`eval`、`innerHTML` 赋值、React 的 `dangerouslySetInnerHTML` JSX 属性、`document.write`
- **C / C++**：`gets`、`sprintf`、`strcpy`、`strcat`、`system`
- **Go**：`exec.Command`、`template.HTML`

每命中一处报一个 Warning，写明触发的名字和一句原因。AST 路径走的，字符串 `"system(rm -rf /)"` 写在日志里不会触发。

`ctx.tree` 不可用时插件直接返回空——比起 grep 一通在字符串、注释里乱报，宁愿沉默。

没配置项——危险 API 名单写死。

### `error_handling`

源码：[`error_handling.rs`](../cha-core/src/plugins/error_handling.rs)

两个独立的 smell 共用一次扫描：

- **`unwrap_abuse`** —— 函数里 `.unwrap()` 或 `.expect(...)` 的次数超过 `max_unwraps_per_function`（默认 3），把这个函数里**每一处** `.unwrap()` / `.expect()` 都报成 Warning。检测走 `(call_expression (field_expression (field_identifier) @method))` 然后比对方法名是不是 `unwrap` / `expect`。
- **`empty_catch`** —— TypeScript / JavaScript 的 `catch` 或 Python 的 `except` 块如果是空的，或者只有 `pass` / 一行注释，报 Warning。Rust 不在这条规则里——`match` 的空分支大多数时候是有意为之。

阈值要抓的是那种"unwrap 的速度快过错误模型设计"的函数。一处 `.unwrap()` 跑在已知必成立的不变量上没问题；同一函数十处就该考虑这个函数本身应该返回 `Result`。

```toml
[plugins.error_handling]
max_unwraps_per_function = 5
```
