# Markdown folder-search microbenchmark — 2026-10-23

**9.986× median paired speedup on the ASCII no-match case; 1.085–32.592× across six cases.** This measures the in-memory `folder::lines_with` search helper, not directory traversal, file I/O, startup, rendering or end-to-end UI latency.

## Change and correctness

ASCII lines use their byte offset directly as the character column. Unicode lines retain the original lowercase/final-sigma folding, then map only the matched prefix back to an original character column. Neither path builds the original per-folded-byte position vector.

`tests/perf/folder-search.rs` freezes the upstream mapping and search loop from `591a8777e8f960fb679272ff2cbbdab03de10b34`. Both wrappers start with a fresh result vector. Full hit records must match on the six inputs and 4,500 additional seeded searches, covering Unicode expansion, sigma, character columns and line boundaries. The existing 500-hit cap is asserted and retained; match-heavy workloads therefore search less of the input.

The added unit test also covers empty input: an empty query on an empty line now returns column zero instead of indexing an empty position vector. The UI rejects trimmed-empty queries before calling this helper.

## Method and results

Linux x86-64, Intel Core i7-13700HX, rustc 1.99.0, standalone `-O`. Three warmups and 30 seeded, randomized baseline/current pairs per case, in one executable on identical generated inputs. Timing includes building and dropping the returned hits. Other host workloads were not stopped.

| Case | Input bytes | Baseline P50 ms | New P50 ms | Median paired speedup |
| --- | ---: | ---: | ---: | ---: |
| ASCII prose, no match | 1,089,000 | 31.5256 | 3.1512 | 9.986× |
| ASCII prose, matches | 1,089,000 | 1.7331 | 0.3687 | 4.667× |
| Unicode notes, no match | 1,034,000 | 30.0605 | 20.5796 | 1.460× |
| Unicode notes, matches | 1,034,000 | 1.6416 | 1.5116 | 1.085× |
| Long generated line, no match | 1,092,000 | 22.2165 | 0.6793 | 32.592× |
| Long generated line, final match | 1,092,004 | 21.6060 | 1.0805 | 20.113× |

Speedups are medians of paired ratios, not ratios of the displayed P50s. Paired bootstrap 95% intervals (10,000 resamples, seed 13), in table order: 9.902–10.057×, 4.572–4.749×, 1.456–1.467×, 1.084–1.090×, 30.661–33.876× and 19.867–20.321×. Intervals describe this run, not cross-machine variation.

A separate no-effect run redirected the `current` wrapper to `baseline`, with the same workloads, warmups, pair count and order. Median paired ratios were 0.999754, 1.001341, 0.999885, 0.997804, 0.999346 and 0.995661. All six control intervals included 1.0.

Results CSV SHA-256: `803ce35aea9aa72f470312b5ff8bbb58971c38d4b15328f02cf0f03e499df209`. Control CSV SHA-256: `d9eec207427df75a16f5ec0e262768eacefa9734fb4709f2ba4a7748bae69803`.

## Reproduce

Choose an output directory with sufficient space, then run from the repository root:

```sh
rustc --edition 2024 -O tests/perf/folder-search.rs -o "$OUTPUT_DIR/folder-search"
"$OUTPUT_DIR/folder-search" > "$OUTPUT_DIR/folder-search.csv"
cargo test --locked --all-features -p ely-mark --bin ely-mark folder::tests
```

For the no-effect control, temporarily replace the benchmark's `current` wrapper body with `baseline(text, query)`, compile to a separate executable and repeat the run. Restore the real wrapper afterward. The normal benchmark checks correctness before timing and writes only timing rows to stdout.

No new GUI or native-platform acceptance was performed for this isolated PR. These numbers are helper-level measurements, not a whole-application speedup.
