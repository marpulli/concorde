use std::cmp::Reverse;
use std::sync::LazyLock;

pub struct PrefixDefinition {
    pub symbol: &'static str,
    pub scale: f64,
    /// Canonical symbol first, followed by accepted alternative spellings.
    pub spellings: &'static [&'static str],
}

impl PrefixDefinition {
    const fn new(symbol: &'static str, scale: f64, spellings: &'static [&'static str]) -> Self {
        Self {
            symbol,
            scale,
            spellings,
        }
    }
}

// Prefixes are definitions, not tokenizer rules. Symbols remain case-sensitive.
const SI_PREFIXES: &[PrefixDefinition] = &[
    PrefixDefinition::new("q", 1e-30, &["q", "quecto"]),
    PrefixDefinition::new("r", 1e-27, &["r", "ronto"]),
    PrefixDefinition::new("y", 1e-24, &["y", "yocto"]),
    PrefixDefinition::new("z", 1e-21, &["z", "zepto"]),
    PrefixDefinition::new("a", 1e-18, &["a", "atto"]),
    PrefixDefinition::new("f", 1e-15, &["f", "femto"]),
    PrefixDefinition::new("p", 1e-12, &["p", "pico"]),
    PrefixDefinition::new("n", 1e-9, &["n", "nano"]),
    PrefixDefinition::new("µ", 1e-6, &["µ", "μ", "u", "micro"]),
    PrefixDefinition::new("m", 1e-3, &["m", "milli"]),
    PrefixDefinition::new("c", 1e-2, &["c", "centi"]),
    PrefixDefinition::new("d", 1e-1, &["d", "deci"]),
    PrefixDefinition::new("da", 1e1, &["da", "deca", "deka"]),
    PrefixDefinition::new("h", 1e2, &["h", "hecto"]),
    PrefixDefinition::new("k", 1e3, &["k", "kilo"]),
    PrefixDefinition::new("M", 1e6, &["M", "mega"]),
    PrefixDefinition::new("G", 1e9, &["G", "giga"]),
    PrefixDefinition::new("T", 1e12, &["T", "tera"]),
    PrefixDefinition::new("P", 1e15, &["P", "peta"]),
    PrefixDefinition::new("E", 1e18, &["E", "exa"]),
    PrefixDefinition::new("Z", 1e21, &["Z", "zetta"]),
    PrefixDefinition::new("Y", 1e24, &["Y", "yotta"]),
    PrefixDefinition::new("R", 1e27, &["R", "ronna"]),
    PrefixDefinition::new("Q", 1e30, &["Q", "quetta"]),
];

static SPELLINGS: LazyLock<Vec<(&str, &PrefixDefinition)>> = LazyLock::new(|| {
    let mut spellings: Vec<_> = SI_PREFIXES
        .iter()
        .flat_map(|prefix| {
            prefix
                .spellings
                .iter()
                .map(move |spelling| (*spelling, prefix))
        })
        .collect();
    spellings.sort_by_key(|(spelling, _)| Reverse(spelling.len()));
    spellings
});

/// Yield matching prefixes longest-first without splitting UTF-8 code points.
/// The caller must still check whether each remainder names a defined unit.
pub fn matches(name: &str) -> impl Iterator<Item = (&'static PrefixDefinition, &str)> {
    SPELLINGS.iter().filter_map(move |(spelling, prefix)| {
        name.strip_prefix(spelling)
            .map(|remainder| (*prefix, remainder))
    })
}
