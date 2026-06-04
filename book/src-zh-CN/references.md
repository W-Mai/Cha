# 学术参考

Cha 的检测器不是凭空拍脑袋。下面是源码中真正引用了的文献——按 detector 分组，每条说明哪条 smell 用到。

## God Class / Brain Method

**M. Lanza, R. Marinescu**. *Object-Oriented Metrics in Practice: Using Software Metrics to Characterize, Evaluate, and Improve the Design of Object-Oriented Systems*. Springer, 2006. doi:[10.1007/3-540-39538-5](https://doi.org/10.1007/3-540-39538-5).

- **第 6.1 章**——`god_class` 的检测策略 `(ATFD > Few) AND (WMC ≥ VeryHigh) AND (TCC < 1/3)`，阈值取自 45 个 Java 项目的统计
- **第 6.2 章**——`brain_method` 的多指标组合：长 + 复杂 + 外部引用多

## Cognitive Complexity

**G. A. Campbell**. *Cognitive Complexity: A new way of measuring understandability*. SonarSource White Paper, 2017. <https://www.sonarsource.com/resources/white-papers/cognitive-complexity/>.

- `cognitive_complexity` 算法基础——衡量"读着累不累"，对嵌套加权惩罚

## Error Handling

**G. Padua, W. Shang**. *Revisiting Exception Handling Practices with Exception Flow Analysis*. Empirical Software Engineering, vol. 23, no. 6, pp. 3337–3383, 2018. doi:[10.1007/s10664-018-9601-8](https://doi.org/10.1007/s10664-018-9601-8).

**A. Rahman, C. Parnin, L. Williams**. *The Seven Sins: Security Smells in Infrastructure as Code Scripts*. ICSE 2019, pp. 164–175. doi:[10.1109/ICSE.2019.00033](https://doi.org/10.1109/ICSE.2019.00033).

- `error_handling` —— 空 catch、unwrap 滥用的检测启发自这两篇

## Hub-Like Dependency

**F. Arcelli Fontana, I. Pigazzini, R. Roveda, M. Zanoni**. *Architectural Smells Detected by Tools: a Catalogue Proposal*. ECSA 2019. doi:[10.1145/3344948.3344982](https://doi.org/10.1145/3344948.3344982).

**R. C. Martin**. *Agile Software Development: Principles, Patterns, and Practices*. Prentice Hall, 2003. ISBN: 978-0135974445. 第 20 章 *Stable Dependencies Principle*。

- `hub_like_dependency` —— 高扇出导致的"枢纽节点"识别 + Stable Dependencies 原则

## Unsafe API

**CWE-676**: Use of Potentially Dangerous Function. <https://cwe.mitre.org/data/definitions/676.html>.

- `unsafe_api` 危险调用清单（eval / exec / system / sprintf / strcpy / strcat / gets / `unsafe` / innerHTML 等）的依据
