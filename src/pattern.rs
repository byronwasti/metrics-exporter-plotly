use regex::Regex;
use std::collections::HashMap;

pub struct PatternGroup {
    patterns: Vec<(Regex, PlotKind)>,
}

impl Default for PatternGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternGroup {
    pub fn new() -> Self {
        Self { patterns: vec![] }
    }

    /// # Panics
    /// Panics on an invalid Regex
    pub fn pattern(mut self, regex: &str, kind: PlotKind) -> Self {
        let regex = Regex::new(regex).unwrap();
        self.patterns.push((regex, kind));
        self
    }

    // TODO: Handle multiple captures
    pub(crate) fn apply(&self, metrics: &[&str]) -> Vec<Vec<(String, PlotKind)>> {
        let mut m: HashMap<_, Vec<(String, PlotKind)>> = HashMap::new();

        for metric in metrics {
            for (re, plot_kind) in &self.patterns {
                let Some(caps) = re.captures(metric) else {
                    continue;
                };

                let cap = caps
                    .iter()
                    .skip(1)
                    .map(|c| c.unwrap().as_str())
                    .take(1)
                    .collect::<Vec<_>>();
                let [cap] = cap[..] else { continue };

                if let Some(v) = m.get_mut(cap) {
                    v.push((metric.to_string(), *plot_kind));
                } else {
                    m.insert(cap, vec![(metric.to_string(), *plot_kind)]);
                }
            }
        }

        m.drain().map(|(_, group)| group).collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlotKind {
    Line,
    Rate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_patterns() {
        let group = PatternGroup::new()
            .pattern(r"(?<scenario>.*)_success", PlotKind::Rate)
            .pattern(r"(?<scenario>.*)_error", PlotKind::Line);

        let metrics = vec![
            "foo_success",
            "bar_success",
            "foo_error",
            "bar_error",
            "baz_drror_",
        ];

        let groups = group.apply(&metrics);

        assert!(groups.contains(&vec![
            ("foo_success".to_string(), PlotKind::Rate),
            ("foo_error".to_string(), PlotKind::Line)
        ]));

        assert!(groups.contains(&vec![
            ("bar_success".to_string(), PlotKind::Rate),
            ("bar_error".to_string(), PlotKind::Line)
        ]));
    }

    #[test]
    fn test_pattern() {
        let re = Regex::new(r"(?<scenario>.*)_success_(?<transaction>.*)").unwrap();

        let cap = re.captures("foo_success_");

        println!("{cap:?}");
    }
}
