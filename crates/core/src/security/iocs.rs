//! Known indicators of compromise, bundled plus an optional user override.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HomePath {
    pub path: String,
    pub malware: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Iocs {
    pub compromised_packages: HashMap<String, Vec<String>>,
    pub package_files: Vec<String>,
    pub install_script_markers: Vec<String>,
    pub home_paths: Vec<HomePath>,
    pub repo_files: Vec<String>,
    pub workflow_files: Vec<String>,
    pub content_markers: Vec<String>,
}

const BUNDLED: &str = include_str!("iocs.json");

impl Iocs {
    pub fn bundled() -> Self {
        serde_json::from_str(BUNDLED).expect("bundled iocs.json is valid")
    }

    /// Bundled list plus `~/.mrclean/iocs.json` if present.
    pub fn load(home: &Path) -> Self {
        let mut iocs = Self::bundled();
        if let Ok(text) = std::fs::read_to_string(home.join(".mrclean/iocs.json")) {
            if let Ok(extra) = serde_json::from_str::<Iocs>(&text) {
                iocs.merge(extra);
            }
        }
        iocs
    }

    fn merge(&mut self, o: Iocs) {
        for (k, v) in o.compromised_packages {
            let e = self.compromised_packages.entry(k).or_default();
            for ver in v {
                if !e.contains(&ver) {
                    e.push(ver);
                }
            }
        }
        self.package_files.extend(o.package_files);
        self.install_script_markers.extend(o.install_script_markers);
        self.home_paths.extend(o.home_paths);
        self.repo_files.extend(o.repo_files);
        self.workflow_files.extend(o.workflow_files);
        self.content_markers.extend(o.content_markers);
    }

    pub fn is_compromised(&self, name: &str, version: &str) -> bool {
        self.compromised_packages.get(name).is_some_and(|v| v.iter().any(|x| x == version))
    }

    pub fn has_marker(&self, text: &str) -> Option<&str> {
        self.content_markers.iter().find(|m| text.contains(m.as_str())).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_parses_and_merges() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join(".mrclean")).unwrap();
        std::fs::write(
            d.path().join(".mrclean/iocs.json"),
            r#"{"compromised_packages":{"chalk":["9.9.9"],"evil-pkg":["1.0.0"]}}"#,
        )
        .unwrap();
        let i = Iocs::load(d.path());
        assert!(i.is_compromised("chalk", "5.6.1"));
        assert!(i.is_compromised("chalk", "9.9.9"));
        assert!(i.is_compromised("evil-pkg", "1.0.0"));
        assert!(!i.is_compromised("chalk", "5.6.2"));
        assert!(!i.home_paths.is_empty());
    }
}
