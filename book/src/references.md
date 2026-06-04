# References

Cha's detectors aren't made up. Where a smell has a published definition or a documented threshold derivation, the source file cites the paper in its module-level doc comment. This page collects those citations.

Only literature that's actually referenced in `cha-core/src/plugins/*.rs` is listed. This isn't a survey of the field.

## Object-Oriented Metrics

**Lanza & Marinescu, *Object-Oriented Metrics in Practice*** — Springer, 2006. doi: [10.1007/3-540-39538-5](https://doi.org/10.1007/3-540-39538-5).

The source for two detectors:

- **`god_class`** uses the Chapter 6.1 detection strategy `(ATFD > Few) AND (WMC ≥ VeryHigh) AND (TCC < 1/3)`. Default thresholds (`ATFD = 5`, `WMC = 47`, `TCC = 0.33`) come from Table A.2, derived from a 45-Java-project corpus.
- **`brain_method`** uses the Chapter 6.2 strategy. Cha runs a three-metric variant `(LOC > 65) AND (CYCLO ≥ 4) AND (NOAV > 7)` because it doesn't track MAXNESTING; the LOC and NOAV thresholds map to "High/2" and "Many" from Table A.2.

## Cognitive Complexity

**G. A. Campbell, *Cognitive Complexity: A new way of measuring understandability*** — SonarSource white paper, 2017. <https://www.sonarsource.com/resources/white-papers/cognitive-complexity/>

Used by **`cognitive_complexity`**. The metric counts branching like cyclomatic complexity but penalises nesting depth and rewards linear structures (a flat `switch` is cheap, a deeply nested `if` is not). Default threshold of 15 is the value the white paper recommends as the warning line for "harder to understand than necessary"; Cha promotes to Error above `2 × threshold`.

## Exception Handling

**G. Padua and W. Shang, *Revisiting Exception Handling Practices with Exception Flow Analysis*** — Empirical Software Engineering 23(6), 2018, pp. 3337–3383. doi: [10.1007/s10664-018-9601-8](https://doi.org/10.1007/s10664-018-9601-8).

**A. Rahman, C. Parnin, L. Williams, *The Seven Sins: Security Smells in Infrastructure as Code Scripts*** — Proc. ICSE 2019, pp. 164–175. doi: [10.1109/ICSE.2019.00033](https://doi.org/10.1109/ICSE.2019.00033).

Both papers underpin **`error_handling`** — empty `catch` / `except` blocks are the canonical "swallow and continue" anti-pattern documented in Padua & Shang; the unwrap/expect abuse rule is the same idea applied to Rust's panic-on-error idiom, with framing borrowed from Rahman et al.'s catalogue of security-relevant error-handling sins.

## Architectural Smells

**F. Arcelli Fontana, I. Pigazzini, R. Roveda, M. Zanoni, *Architectural Smells Detected by Tools: a Catalogue Proposal*** — Proc. ECSA 2019. doi: [10.1145/3344948.3344982](https://doi.org/10.1145/3344948.3344982).

**R. C. Martin, *Agile Software Development: Principles, Patterns, and Practices*** — Prentice Hall, 2003. ISBN: 978-0135974445. Chapter 20: Stable Dependencies Principle.

Together these inform **`hub_like_dependency`**. Arcelli Fontana et al. catalogue the hub-like-dependency smell (a module with disproportionate fan-in or fan-out acts as an architectural single point of contact); Martin's Stable Dependencies Principle gives the design rationale for why high fan-out into volatile modules is bad. Cha's default `max_imports = 20` is a fan-out threshold — fan-in is reported separately by `coupling`.

## Dangerous APIs

**CWE-676: Use of Potentially Dangerous Function** — MITRE Common Weakness Enumeration. <https://cwe.mitre.org/data/definitions/676.html>

Used by **`unsafe_api`** as the rationale for flagging `eval`, `exec`, `system`, `sprintf`, `strcpy`, `strcat`, `gets`, Rust `unsafe`, and DOM sinks like `innerHTML` / `dangerouslySetInnerHTML`. CWE-676 is the canonical catalogue entry for "this call has a safer replacement; use it"; Cha's role is to surface call sites, not to certify any particular alternative.

## Dead Code

**`dead_code`** has no published-paper reference — the detection logic (in-file AST identifier scan + cross-file call graph + C/C++ token-pasting macro expansion) is implementation-defined. It's listed here only to make clear the omission is deliberate, not an oversight.
