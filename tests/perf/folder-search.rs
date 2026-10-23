//! In-memory baseline/current search benchmark.

#[path = "../../apps/ely-mark/src/folder.rs"]
#[allow(dead_code)] // This harness exercises search, not directory traversal.
mod folder;

use std::{hint::black_box, path::Path, time::Instant};

fn baseline_column(line: &str, query: &str) -> Option<usize> {
    let (mut folded, mut places) = (String::new(), Vec::new());
    for (place, ch) in line.chars().enumerate() {
        for low in ch
            .to_lowercase()
            .map(|low| if low == 'ς' { 'σ' } else { low })
        {
            folded.push(low);
            places.extend(std::iter::repeat_n(place, low.len_utf8()));
        }
    }
    folded.find(query).map(|at| places[at])
}

fn baseline(text: &str, query: &str) -> Vec<folder::Hit> {
    let query: String = query
        .chars()
        .flat_map(|ch| {
            ch.to_lowercase()
                .map(|low| if low == 'ς' { 'σ' } else { low })
        })
        .collect();
    let mut hits = Vec::new();
    for (at, line) in text.lines().enumerate() {
        if hits.len() == folder::MOST_HITS {
            return hits;
        }
        if let Some(column) = baseline_column(line, &query) {
            let shown: String = line.trim().chars().take(160).collect();
            hits.push(folder::Hit {
                file: Path::new("notes.md").to_path_buf(),
                line: at + 1,
                column,
                text: shown,
            });
        }
    }
    hits
}

fn current(text: &str, query: &str) -> Vec<folder::Hit> {
    let mut hits = Vec::new();
    folder::lines_with(Path::new("notes.md"), text, query, &mut hits);
    hits
}

fn run(search: fn(&str, &str) -> Vec<folder::Hit>, text: &str, query: &str) -> u128 {
    let start = Instant::now();
    black_box(search(black_box(text), black_box(query)));
    start.elapsed().as_nanos()
}

fn verify() {
    assert_eq!(folder::MOST_HITS, 500);
    let alphabet = [
        'a', 'A', 'İ', 'i', '\u{307}', 'Σ', 'ς', 'σ', '中', '文', '😀', ' ', '\n', '\r',
    ];
    let queries = ["a", "İ", "i", "\u{307}", "ΟΣ", "ς", "中", "😀", "missing"];
    let mut seed = 7u64;
    for _ in 0..500 {
        let text: String = (0..240)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                alphabet[(seed >> 33) as usize % alphabet.len()]
            })
            .collect();
        for query in queries {
            assert_eq!(baseline(&text, query), current(&text, query));
        }
    }
    eprintln!("PASS: 4500 seeded differential searches match the original implementation");
}

fn main() {
    verify();
    let mut order = 17u64;
    let prose = "## Project notes\nThe review covers document layout, rendering, offline storage, and compatibility.\n";
    let han = "## 项目笔记\n文档排版、离线存储与兼容性检查。İİ née Word ΟΣ οσ ος\n";
    let long = "An inline generated report contains ordinary English words and numbers 12345. ";
    let cases = [
        ("ascii-miss", prose.repeat(11_000), "unmatched"),
        ("ascii-hits", prose.repeat(11_000), "REVIEW"),
        ("unicode-miss", han.repeat(11_000), "unmatched"),
        ("unicode-hits", han.repeat(11_000), "word"),
        ("long-line-miss", long.repeat(14_000), "unmatched"),
        (
            "long-line-hit",
            format!("{} END", long.repeat(14_000)),
            "end",
        ),
    ];
    println!("case,bytes,round,baseline_ns,current_ns");
    for (name, text, query) in cases {
        assert_eq!(baseline(&text, query), current(&text, query), "{name}");
        for _ in 0..3 {
            run(baseline, &text, query);
            run(current, &text, query);
        }
        for round in 0..30 {
            order = order.wrapping_mul(6364136223846793005).wrapping_add(1);
            let (a, b) = if order >> 63 == 0 {
                (run(baseline, &text, query), run(current, &text, query))
            } else {
                let b = run(current, &text, query);
                (run(baseline, &text, query), b)
            };
            println!("{name},{},{round},{a},{b}", text.len());
        }
    }
}
