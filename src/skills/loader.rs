//! Skill loader — loads and manages markdown-based skill files
//!
//! Skill file locations:
//!   ~/.claude/skills/*.md  — global skills
//!   ./.claude/skills/*.md  — project skills
//!
//! Skill file format (markdown):
//! ```markdown
//! ---
//! name: commit
//! description: Auto-generate AI commit message
//! args: [message]
//! ---
//!
//! Review the following git diff and write a commit message:
//! {{message}}
//! ```

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

// ─── Skill definition ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,       // parameter name list
    pub prompt_template: String, // template with {{arg}} placeholders
    pub source_path: String,     // path of loaded file
}

impl Skill {
    /// Apply args to produce final prompt
    pub fn expand(&self, args: &[&str]) -> String {
        let mut prompt = self.prompt_template.clone();
        // Positional args: {{0}}, {{1}} or named args: {{arg_name}}
        for (i, (param, value)) in self.args.iter().zip(args.iter()).enumerate() {
            prompt = prompt.replace(&format!("{{{{{}}}}}", param), value);
            prompt = prompt.replace(&format!("{{{{{}}}}}", i), value);
        }
        // Remaining positional args
        for (i, value) in args.iter().enumerate() {
            prompt = prompt.replace(&format!("{{{{{}}}}}", i), value);
        }
        // Join remaining args as one string and substitute {{args}}
        let all_args = args.join(" ");
        prompt = prompt.replace("{{args}}", &all_args);
        prompt
    }
}

// ─── YAML frontmatter Parsing ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: Option<String>,
    args: Option<Vec<String>>,
}

fn parse_skill_file(content: &str, path: &str) -> Option<Skill> {
    // Check if frontmatter starts with ---
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end = after_first.find("---")?;
    let frontmatter_str = after_first[..end].trim();
    let body = after_first[end + 3..].trim().to_string();

    // Simple YAML parsing (without serde_yaml)
    let fm = parse_simple_yaml(frontmatter_str)?;

    Some(Skill {
        name: fm.name,
        description: fm.description.unwrap_or_default(),
        args: fm.args.unwrap_or_default(),
        prompt_template: body,
        source_path: path.to_string(),
    })
}

/// Simple YAML parsing without serde_yaml
fn parse_simple_yaml(yaml: &str) -> Option<SkillFrontmatter> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut current_list_key: Option<String> = None;
    let mut list_items: Vec<String> = Vec::new();

    for line in yaml.lines() {
        if line.trim().is_empty() { continue; }

        // List item
        if let Some(item) = line.trim().strip_prefix("- ") {
            list_items.push(item.trim_matches('"').to_string());
            continue;
        }

        // Save previous list
        if let Some(key) = current_list_key.take() {
            map.insert(key, serde_json::to_string(&list_items).unwrap_or_default());
            list_items.clear();
        }

        // key: value
        if let Some((key, value)) = line.split_once(':') {
            let k = key.trim().to_string();
            let v = value.trim().trim_matches('"').to_string();
            if v.is_empty() {
                // Next line may be a list
                current_list_key = Some(k);
            } else {
                map.insert(k, v);
            }
        }
    }

    // Save last list
    if let Some(key) = current_list_key {
        map.insert(key, serde_json::to_string(&list_items).unwrap_or_default());
    }

    let name = map.get("name")?.clone();
    let description = map.get("description").cloned();
    let args: Option<Vec<String>> = map.get("args")
        .and_then(|s| serde_json::from_str(s).ok());

    Some(SkillFrontmatter { name, description, args })
}

// ─── Skill registry ────────────────────────────────────────────────────────

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self { skills: HashMap::new() }
    }

    /// Auto-load from global and project skill directories
    pub fn load_all(&mut self) -> usize {
        // Global: ~/.claude/skills/
        if let Ok(home) = std::env::var("HOME") {
            let global_dir = std::path::PathBuf::from(home).join(".claude").join("skills");
            self.load_from_dir(&global_dir);
        }

        // Project: ./.claude/skills/
        let project_dir = std::path::PathBuf::from(".claude").join("skills");
        self.load_from_dir(&project_dir);

        self.skills.len()
    }

    fn load_from_dir(&mut self, dir: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(skill) = parse_skill_file(&content, &path_str) {
                        self.skills.insert(skill.name.clone(), skill);
                    }
                }
            }
        }
    }

    /// Find skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Full skill list
    pub fn all(&self) -> Vec<&Skill> {
        let mut list: Vec<&Skill> = self.skills.values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Number of skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Generate skill descriptions to add to AI system prompt
    pub fn descriptions_for_prompt(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        let mut lines = vec![
            "\n## Registered skills".to_string(),
            "Execute skill: /skill <name> [args...]".to_string(),
            String::new(),
        ];
        for skill in self.all() {
            let args_str = if skill.args.is_empty() {
                String::new()
            } else {
                format!(" (args: {})", skill.args.join(", "))
            };
            lines.push(format!("- **{}**{}: {}", skill.name, args_str, skill.description));
        }
        lines.join("\n")
    }

    /// Add skill inline (for testing and programmatic use)
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Create new skill file in directory
    pub fn create_skill_file(name: &str, description: &str, args: &[&str], template: &str) -> Result<String> {
        let dir = std::path::PathBuf::from(".claude").join("skills");
        std::fs::create_dir_all(&dir)?;

        let args_yaml = if args.is_empty() {
            String::new()
        } else {
            format!("args:\n{}\n", args.iter().map(|a| format!("  - {}", a)).collect::<Vec<_>>().join("\n"))
        };

        let content = format!(
            "---\nname: {}\ndescription: {}\n{}---\n\n{}",
            name, description, args_yaml, template
        );

        let path = dir.join(format!("{}.md", name));
        std::fs::write(&path, &content)?;
        Ok(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(name: &str, args: &[&str], template: &str) -> Skill {
        Skill {
            name: name.to_string(),
            description: "test skill".to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            prompt_template: template.to_string(),
            source_path: "test.md".to_string(),
        }
    }

    #[test]
    fn skill_expand_named_args() {
        let skill = make_skill("commit", &["message"], "Write a commit message:\n{{message}}");
        let result = skill.expand(&["Fix bug in login"]);
        assert_eq!(result, "Write a commit message:\nFix bug in login");
    }

    #[test]
    fn skill_expand_positional_index() {
        let skill = make_skill("greet", &[], "Hello {{0}}, this is {{1}}.");
        let result = skill.expand(&["Alice", "Nice to meet you"]);
        assert_eq!(result, "Hello Hong Gildong, this is Nice to meet you.");
    }

    #[test]
    fn skill_expand_all_args_placeholder() {
        let skill = make_skill("echo", &[], "Input: {{args}}");
        let result = skill.expand(&["hello", "world"]);
        assert_eq!(result, "Input: hello world");
    }

    #[test]
    fn skill_expand_no_args_returns_template() {
        let skill = make_skill("noop", &[], "Empty template");
        let result = skill.expand(&[]);
        assert_eq!(result, "Empty template");
    }

    #[test]
    fn parse_skill_file_basic() {
        let content = "---\nname: test\ndescription: test skill\n---\n\nprompt content";
        let skill = parse_skill_file(content, "test.md").unwrap();
        assert_eq!(skill.name, "test");
        assert_eq!(skill.description, "test skill");
        assert_eq!(skill.prompt_template, "prompt content");
        assert!(skill.args.is_empty());
    }

    #[test]
    fn parse_skill_file_with_args() {
        let content = "---\nname: review\ndescription: code review\nargs:\n  - code\n  - lang\n---\n\nReview {{code}} in {{lang}}";
        let skill = parse_skill_file(content, "review.md").unwrap();
        assert_eq!(skill.args, vec!["code", "lang"]);
        assert!(skill.prompt_template.contains("{{code}}"));
    }

    #[test]
    fn parse_skill_file_no_frontmatter_returns_none() {
        let content = "Plain markdown text.";
        assert!(parse_skill_file(content, "bad.md").is_none());
    }

    #[test]
    fn registry_register_and_get() {
        let mut registry = SkillRegistry::new();
        let skill = make_skill("hello", &[], "Hello");
        registry.register(skill);
        assert!(registry.get("hello").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn registry_list_all() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("a", &[], "A"));
        registry.register(make_skill("b", &[], "B"));
        let all = registry.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn registry_describe_not_empty_when_has_skills() {
        let mut registry = SkillRegistry::new();
        registry.register(make_skill("demo", &["x"], "demo prompt"));
        let desc = registry.descriptions_for_prompt();
        assert!(desc.contains("demo"));
        assert!(desc.contains("x"));
    }
}
