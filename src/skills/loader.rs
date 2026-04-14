//! Skill 로더 — 마크다운 기반 skill 파일을 로드하고 관리
//!
//! Skill 파일 위치:
//!   ~/.claude/skills/*.md  — 전역 스킬
//!   ./.claude/skills/*.md  — 프로젝트 스킬
//!
//! Skill 파일 형식 (마크다운):
//! ```markdown
//! ---
//! name: commit
//! description: AI 커밋 메시지 자동 생성
//! args: [message]
//! ---
//!
//! 다음 git diff를 보고 커밋 메시지를 작성하세요:
//! {{message}}
//! ```

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

// ─── Skill 정의 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub args: Vec<String>,       // 파라미터 이름 목록
    pub prompt_template: String, // {{arg}} 형태의 템플릿
    pub source_path: String,     // 로드된 파일 경로
}

impl Skill {
    /// 인자를 적용하여 최종 프롬프트 생성
    pub fn expand(&self, args: &[&str]) -> String {
        let mut prompt = self.prompt_template.clone();
        // 위치 인자: {{0}}, {{1}} 또는 이름 인자: {{arg_name}}
        for (i, (param, value)) in self.args.iter().zip(args.iter()).enumerate() {
            prompt = prompt.replace(&format!("{{{{{}}}}}", param), value);
            prompt = prompt.replace(&format!("{{{{{}}}}}", i), value);
        }
        // 남은 위치 인자
        for (i, value) in args.iter().enumerate() {
            prompt = prompt.replace(&format!("{{{{{}}}}}", i), value);
        }
        // 나머지 args를 한 문자열로 합쳐서 {{args}} 치환
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
    // frontmatter가 ---로 시작하는지 확인
    let content = content.trim();
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end = after_first.find("---")?;
    let frontmatter_str = after_first[..end].trim();
    let body = after_first[end + 3..].trim().to_string();

    // 간단한 YAML Parsing (serde_yaml 없이)
    let fm = parse_simple_yaml(frontmatter_str)?;

    Some(Skill {
        name: fm.name,
        description: fm.description.unwrap_or_default(),
        args: fm.args.unwrap_or_default(),
        prompt_template: body,
        source_path: path.to_string(),
    })
}

/// serde_yaml 없이 간단한 YAML Parsing
fn parse_simple_yaml(yaml: &str) -> Option<SkillFrontmatter> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut current_list_key: Option<String> = None;
    let mut list_items: Vec<String> = Vec::new();

    for line in yaml.lines() {
        if line.trim().is_empty() { continue; }

        // 리스트 아이템
        if let Some(item) = line.trim().strip_prefix("- ") {
            list_items.push(item.trim_matches('"').to_string());
            continue;
        }

        // 이전 리스트 저장
        if let Some(key) = current_list_key.take() {
            map.insert(key, serde_json::to_string(&list_items).unwrap_or_default());
            list_items.clear();
        }

        // 키: 값
        if let Some((key, value)) = line.split_once(':') {
            let k = key.trim().to_string();
            let v = value.trim().trim_matches('"').to_string();
            if v.is_empty() {
                // 다음 줄이 리스트일 수 있음
                current_list_key = Some(k);
            } else {
                map.insert(k, v);
            }
        }
    }

    // 마지막 리스트 저장
    if let Some(key) = current_list_key {
        map.insert(key, serde_json::to_string(&list_items).unwrap_or_default());
    }

    let name = map.get("name")?.clone();
    let description = map.get("description").cloned();
    let args: Option<Vec<String>> = map.get("args")
        .and_then(|s| serde_json::from_str(s).ok());

    Some(SkillFrontmatter { name, description, args })
}

// ─── Skill 레지스트리 ────────────────────────────────────────────────────────

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self { skills: HashMap::new() }
    }

    /// 전역 및 프로젝트 스킬 디렉토리에서 자동 로드
    pub fn load_all(&mut self) -> usize {
        // 전역: ~/.claude/skills/
        if let Ok(home) = std::env::var("HOME") {
            let global_dir = std::path::PathBuf::from(home).join(".claude").join("skills");
            self.load_from_dir(&global_dir);
        }

        // 프로젝트: ./.claude/skills/
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

    /// 이름으로 스킬 조회
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// 전체 스킬 목록
    pub fn all(&self) -> Vec<&Skill> {
        let mut list: Vec<&Skill> = self.skills.values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// 스킬 수
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// AI 시스템 프롬프트에 추가할 스킬 설명
    pub fn descriptions_for_prompt(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        let mut lines = vec![
            "\n## 등록된 스킬".to_string(),
            "스킬 실행: /skill <name> [args...]".to_string(),
            String::new(),
        ];
        for skill in self.all() {
            let args_str = if skill.args.is_empty() {
                String::new()
            } else {
                format!(" (인자: {})", skill.args.join(", "))
            };
            lines.push(format!("- **{}**{}: {}", skill.name, args_str, skill.description));
        }
        lines.join("\n")
    }

    /// 인라인으로 스킬 추가 (테스트 및 프로그래매틱 용)
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// 디렉토리에 새 스킬 파일 생성
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
        let skill = make_skill("commit", &["message"], "커밋 메시지를 작성하세요:\n{{message}}");
        let result = skill.expand(&["Fix bug in login"]);
        assert_eq!(result, "커밋 메시지를 작성하세요:\nFix bug in login");
    }

    #[test]
    fn skill_expand_positional_index() {
        let skill = make_skill("greet", &[], "안녕하세요 {{0}}님, {{1}}입니다.");
        let result = skill.expand(&["홍길동", "반갑습니다"]);
        assert_eq!(result, "안녕하세요 홍길동님, 반갑습니다입니다.");
    }

    #[test]
    fn skill_expand_all_args_placeholder() {
        let skill = make_skill("echo", &[], "입력: {{args}}");
        let result = skill.expand(&["hello", "world"]);
        assert_eq!(result, "입력: hello world");
    }

    #[test]
    fn skill_expand_no_args_returns_template() {
        let skill = make_skill("noop", &[], "빈 템플릿");
        let result = skill.expand(&[]);
        assert_eq!(result, "빈 템플릿");
    }

    #[test]
    fn parse_skill_file_basic() {
        let content = "---\nname: test\ndescription: 테스트 스킬\n---\n\n프롬프트 내용";
        let skill = parse_skill_file(content, "test.md").unwrap();
        assert_eq!(skill.name, "test");
        assert_eq!(skill.description, "테스트 스킬");
        assert_eq!(skill.prompt_template, "프롬프트 내용");
        assert!(skill.args.is_empty());
    }

    #[test]
    fn parse_skill_file_with_args() {
        let content = "---\nname: review\ndescription: 코드 리뷰\nargs:\n  - code\n  - lang\n---\n\n{{code}}를 {{lang}}로 리뷰";
        let skill = parse_skill_file(content, "review.md").unwrap();
        assert_eq!(skill.args, vec!["code", "lang"]);
        assert!(skill.prompt_template.contains("{{code}}"));
    }

    #[test]
    fn parse_skill_file_no_frontmatter_returns_none() {
        let content = "그냥 마크다운 텍스트입니다.";
        assert!(parse_skill_file(content, "bad.md").is_none());
    }

    #[test]
    fn registry_register_and_get() {
        let mut registry = SkillRegistry::new();
        let skill = make_skill("hello", &[], "안녕");
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
