/// 프로젝트 스캐폴딩 툴
///
/// 다양한 언어/프레임워크의 프로젝트 구조를 자동 생성
/// (cargo new, npm create, pip, Django 등 활용)

use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;
use std::time::Duration;

const SCAFFOLD_TIMEOUT: u64 = 120;

/// cargo 실행 파일 경로 탐색 (PATH에 없으면 ~/.cargo/bin/cargo)
fn find_cargo() -> String {
    // PATH에서 찾기
    if Command::new("cargo").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false) {
        return "cargo".to_string();
    }
    // ~/.cargo/bin/cargo
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let cargo_path = format!("{}/.cargo/bin/cargo", home);
    if Path::new(&cargo_path).exists() {
        return cargo_path;
    }
    // 환경변수 CARGO_HOME
    if let Ok(ch) = std::env::var("CARGO_HOME") {
        let p = format!("{}/bin/cargo", ch);
        if Path::new(&p).exists() { return p; }
    }
    "cargo".to_string()
}

#[derive(Debug)]
pub struct ScaffoldResult {
    pub output: String,
    #[allow(dead_code)]
    pub success: bool,
    #[allow(dead_code)]
    pub path: String,
}

impl std::fmt::Display for ScaffoldResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
    }
}

// ─── 메인 스캐폴딩 함수 ───────────────────────────────────────────────────────

/// 프로젝트 생성
/// project_type: rust, python, node, typescript, react, vue, next, django, flask, fastapi,
///               go, java-spring, kotlin-spring, express, deno, cpp
pub fn project_init(project_type: &str, name: &str, path: &str) -> Result<ScaffoldResult> {
    let target_dir = if path.is_empty() || path == "." {
        name.to_string()
    } else {
        format!("{}/{}", path.trim_end_matches('/'), name)
    };

    match project_type.to_lowercase().as_str() {
        "rust" => scaffold_rust(name, &target_dir),
        "rust-lib" => scaffold_rust_lib(name, &target_dir),
        "python" | "py" => scaffold_python(name, &target_dir),
        "node" | "nodejs" => scaffold_node(name, &target_dir, false),
        "typescript" | "ts" | "ts-node" => scaffold_node(name, &target_dir, true),
        "react" => scaffold_react(name, &target_dir, false),
        "react-ts" | "react-typescript" => scaffold_react(name, &target_dir, true),
        "vue" => scaffold_vue(name, &target_dir),
        "next" | "nextjs" => scaffold_next(name, &target_dir),
        "django" => scaffold_django(name, &target_dir),
        "flask" => scaffold_flask(name, &target_dir),
        "fastapi" => scaffold_fastapi(name, &target_dir),
        "go" | "golang" => scaffold_go(name, &target_dir),
        "express" => scaffold_express(name, &target_dir),
        "cpp" | "c++" => scaffold_cpp(name, &target_dir),
        "deno" => scaffold_deno(name, &target_dir),
        other => anyhow::bail!(
            "지원하지 않는 프로젝트 타입: '{}'\n지원 타입: rust, rust-lib, python, node, typescript, react, react-ts, vue, next, django, flask, fastapi, go, express, cpp, deno",
            other
        ),
    }
}

// ─── Rust ────────────────────────────────────────────────────────────────────

fn scaffold_rust(name: &str, path: &str) -> Result<ScaffoldResult> {
    let cargo = find_cargo();
    let out = run_cmd(&[&cargo, "new", "--bin", path])?;

    // Cargo.toml에 공통 의존성 추가
    let cargo_toml_path = format!("{}/Cargo.toml", path);
    if Path::new(&cargo_toml_path).exists() {
        let content = std::fs::read_to_string(&cargo_toml_path)?;
        let updated = format!("{}\n[dependencies]\nanyhow = \"1.0\"\nserde = {{ version = \"1\", features = [\"derive\"] }}\n", content);
        let _ = std::fs::write(&cargo_toml_path, updated);
    }

    // README.md 생성
    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\nRust 프로젝트\n\n## 빌드\n\n```bash\ncargo build\n```\n\n## 실행\n\n```bash\ncargo run\n```\n\n## 테스트\n\n```bash\ncargo test\n```\n",
        name
    ));

    // src/lib.rs 없으면 스킵
    let result = format!("✅ Rust 프로젝트 생성: {}\n{}", path, out);
    Ok(ScaffoldResult { output: result, success: true, path: path.to_string() })
}

fn scaffold_rust_lib(name: &str, path: &str) -> Result<ScaffoldResult> {
    let cargo = find_cargo();
    let out = run_cmd(&[&cargo, "new", "--lib", path])?;
    write_file_safe(&format!("{}/README.md", path), &format!("# {}\n\nRust 라이브러리 크레이트\n", name));
    Ok(ScaffoldResult {
        output: format!("✅ Rust 라이브러리 생성: {}\n{}", path, out),
        success: true,
        path: path.to_string(),
    })
}

// ─── Python ──────────────────────────────────────────────────────────────────

fn scaffold_python(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(path).context("디렉토리 생성 실패")?;
    std::fs::create_dir_all(&format!("{}/src", path))?;
    std::fs::create_dir_all(&format!("{}/tests", path))?;

    // main.py
    write_file_safe(&format!("{}/main.py", path),
        "#!/usr/bin/env python3\n\"\"\"Entry point.\"\"\"\n\n\ndef main() -> None:\n    print(\"Hello, World!\")\n\n\nif __name__ == \"__main__\":\n    main()\n"
    );

    // pyproject.toml
    write_file_safe(&format!("{}/pyproject.toml", path), &format!(
        "[build-system]\nrequires = [\"setuptools>=68\"]\nbuild-backend = \"setuptools.backends.legacy:build\"\n\n[project]\nname = \"{}\"\nversion = \"0.1.0\"\ndescription = \"\"\nrequires-python = \">=3.10\"\n",
        name
    ));

    // requirements.txt
    write_file_safe(&format!("{}/requirements.txt", path), "# 의존성 목록\n");

    // .gitignore
    write_file_safe(&format!("{}/.gitignore", path),
        "__pycache__/\n*.pyc\n*.pyo\n.env\nvenv/\n.venv/\ndist/\nbuild/\n*.egg-info/\n.pytest_cache/\n"
    );

    // tests/__init__.py
    write_file_safe(&format!("{}/tests/__init__.py", path), "");
    write_file_safe(&format!("{}/tests/test_main.py", path),
        "\"\"\"Tests for main module.\"\"\"\nfrom main import main\n\n\ndef test_main():\n    main()  # should not raise\n"
    );

    // README
    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\n## 설치\n\n```bash\npython3 -m venv .venv\nsource .venv/bin/activate\npip install -r requirements.txt\n```\n\n## 실행\n\n```bash\npython main.py\n```\n\n## 테스트\n\n```bash\npytest\n```\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ Python 프로젝트 생성: {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Node.js / TypeScript ────────────────────────────────────────────────────

fn scaffold_node(name: &str, path: &str, typescript: bool) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(path)?;
    std::fs::create_dir_all(&format!("{}/src", path))?;

    // package.json
    let pkg_json = if typescript {
        format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "",
  "main": "dist/index.js",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/index.js",
    "dev": "ts-node src/index.ts",
    "test": "jest"
  }},
  "dependencies": {{}},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0",
    "ts-node": "^10.9.0",
    "jest": "^29.0.0",
    "@types/jest": "^29.0.0",
    "ts-jest": "^29.0.0"
  }}
}}
"#, name)
    } else {
        format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "",
  "main": "src/index.js",
  "scripts": {{
    "start": "node src/index.js",
    "test": "jest"
  }},
  "dependencies": {{}},
  "devDependencies": {{
    "jest": "^29.0.0"
  }}
}}
"#, name)
    };
    write_file_safe(&format!("{}/package.json", path), &pkg_json);

    if typescript {
        // tsconfig.json
        write_file_safe(&format!("{}/tsconfig.json", path), r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
"#);
        write_file_safe(&format!("{}/src/index.ts", path),
            "function main(): void {\n  console.log('Hello, World!');\n}\n\nmain();\n"
        );
    } else {
        write_file_safe(&format!("{}/src/index.js", path),
            "function main() {\n  console.log('Hello, World!');\n}\n\nmain();\n"
        );
    }

    // .gitignore
    write_file_safe(&format!("{}/.gitignore", path),
        "node_modules/\ndist/\n.env\n*.log\ncoverage/\n"
    );
    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\n## 설치\n\n```bash\nnpm install\n```\n\n## 실행\n\n```bash\nnpm start\n```\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ {} 프로젝트 생성: {}\n(npm install 실행 필요)", if typescript { "TypeScript" } else { "Node.js" }, path),
        success: true,
        path: path.to_string(),
    })
}

// ─── React ────────────────────────────────────────────────────────────────────

fn scaffold_react(name: &str, path: &str, typescript: bool) -> Result<ScaffoldResult> {
    // npm create vite 시도
    let template = if typescript { "react-ts" } else { "react" };
    let parent = Path::new(path).parent()
        .and_then(|p| p.to_str())
        .unwrap_or(".");

    let out = run_cmd_in(&[
        "npm", "create", "vite@latest", name, "--", "--template", template
    ], parent);

    if let Ok(o) = out {
        return Ok(ScaffoldResult {
            output: format!("✅ React 프로젝트 생성: {}\n{}\n\ncd {}\nnpm install\nnpm run dev", path, o, name),
            success: true,
            path: path.to_string(),
        });
    }

    // 폴백: 수동 생성
    std::fs::create_dir_all(&format!("{}/src", path))?;
    std::fs::create_dir_all(&format!("{}/public", path))?;

    let ext = if typescript { "tsx" } else { "jsx" };
    write_file_safe(&format!("{}/src/App.{}", path, ext), &format!(
        "{}function App() {{\n  return (\n    <div className=\"App\">\n      <h1>{}</h1>\n    </div>\n  );\n}}\n\nexport default App;\n",
        if typescript { "" } else { "" }, name
    ));
    write_file_safe(&format!("{}/src/main.{}", path, ext),
        "import React from 'react';\nimport ReactDOM from 'react-dom/client';\nimport App from './App';\n\nReactDOM.createRoot(document.getElementById('root')!).render(\n  <React.StrictMode>\n    <App />\n  </React.StrictMode>\n);\n"
    );
    write_file_safe(&format!("{}/index.html", path), &format!(
        "<!DOCTYPE html>\n<html lang=\"ko\">\n<head>\n  <meta charset=\"UTF-8\" />\n  <title>{}</title>\n</head>\n<body>\n  <div id=\"root\"></div>\n</body>\n</html>\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ React 프로젝트 생성 (수동): {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Vue ─────────────────────────────────────────────────────────────────────

fn scaffold_vue(name: &str, path: &str) -> Result<ScaffoldResult> {
    let parent = Path::new(path).parent()
        .and_then(|p| p.to_str()).unwrap_or(".");
    let out = run_cmd_in(&["npm", "create", "vue@latest", name], parent);

    if let Ok(o) = out {
        return Ok(ScaffoldResult {
            output: format!("✅ Vue 프로젝트 생성: {}\n{}", path, o),
            success: true,
            path: path.to_string(),
        });
    }

    // 폴백: Vite Vue 템플릿
    let _ = run_cmd_in(&["npm", "create", "vite@latest", name, "--", "--template", "vue"], parent);
    Ok(ScaffoldResult {
        output: format!("✅ Vue 프로젝트 생성: {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Next.js ──────────────────────────────────────────────────────────────────

fn scaffold_next(name: &str, path: &str) -> Result<ScaffoldResult> {
    let parent = Path::new(path).parent()
        .and_then(|p| p.to_str()).unwrap_or(".");
    let out = run_cmd_in(&[
        "npx", "create-next-app@latest", name,
        "--typescript", "--tailwind", "--eslint", "--app", "--src-dir", "--import-alias", "@/*"
    ], parent);

    match out {
        Ok(o) => Ok(ScaffoldResult {
            output: format!("✅ Next.js 프로젝트 생성: {}\n{}", path, o),
            success: true,
            path: path.to_string(),
        }),
        Err(e) => anyhow::bail!("Next.js 프로젝트 생성 실패: {}\nnpx 및 Node.js 설치 필요", e),
    }
}

// ─── Django ──────────────────────────────────────────────────────────────────

fn scaffold_django(name: &str, path: &str) -> Result<ScaffoldResult> {
    let parent = Path::new(path).parent()
        .and_then(|p| p.to_str()).unwrap_or(".");

    // django-admin startproject
    let out = run_cmd_in(&["django-admin", "startproject", name, path], parent);

    let result_msg = match out {
        Ok(o) => format!("✅ Django 프로젝트 생성: {}\n{}", path, o),
        Err(_) => {
            // pip로 django 설치 후 재시도
            let _ = run_cmd(&["python3", "-m", "pip", "install", "django"]);
            let o = run_cmd_in(&["django-admin", "startproject", name, path], parent)?;
            format!("✅ Django 프로젝트 생성: {}\n{}", path, o)
        }
    };

    // requirements.txt
    write_file_safe(&format!("{}/requirements.txt", path),
        "Django>=4.2\ndjangorestframework>=3.14\ndjango-cors-headers>=4.0\npsycopg2-binary>=2.9\npython-decouple>=3.8\n"
    );

    // .env 예시
    write_file_safe(&format!("{}/.env.example", path),
        "DEBUG=True\nSECRET_KEY=your-secret-key-here\nDATABASE_URL=sqlite:///db.sqlite3\nALLOWED_HOSTS=localhost,127.0.0.1\n"
    );

    Ok(ScaffoldResult {
        output: format!("{}\n\n실행:\n  python manage.py migrate\n  python manage.py runserver", result_msg),
        success: true,
        path: path.to_string(),
    })
}

// ─── Flask ───────────────────────────────────────────────────────────────────

fn scaffold_flask(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(&format!("{}/app", path))?;
    std::fs::create_dir_all(&format!("{}/app/routes", path))?;
    std::fs::create_dir_all(&format!("{}/app/models", path))?;
    std::fs::create_dir_all(&format!("{}/app/static", path))?;
    std::fs::create_dir_all(&format!("{}/app/templates", path))?;
    std::fs::create_dir_all(&format!("{}/tests", path))?;

    // app/__init__.py
    write_file_safe(&format!("{}/app/__init__.py", path),
        "from flask import Flask\nfrom flask_sqlalchemy import SQLAlchemy\n\ndb = SQLAlchemy()\n\n\ndef create_app(config=None):\n    app = Flask(__name__)\n    app.config.from_object(config or 'app.config.DevelopmentConfig')\n    db.init_app(app)\n    from app.routes.main import main_bp\n    app.register_blueprint(main_bp)\n    return app\n"
    );

    // app/config.py
    write_file_safe(&format!("{}/app/config.py", path),
        "import os\n\n\nclass Config:\n    SECRET_KEY = os.environ.get('SECRET_KEY', 'dev-secret')\n    SQLALCHEMY_TRACK_MODIFICATIONS = False\n\n\nclass DevelopmentConfig(Config):\n    DEBUG = True\n    SQLALCHEMY_DATABASE_URI = 'sqlite:///dev.db'\n\n\nclass ProductionConfig(Config):\n    DEBUG = False\n    SQLALCHEMY_DATABASE_URI = os.environ.get('DATABASE_URL')\n"
    );

    // app/routes/main.py
    write_file_safe(&format!("{}/app/routes/main.py", path),
        "from flask import Blueprint, jsonify\n\nmain_bp = Blueprint('main', __name__)\n\n\n@main_bp.route('/')\ndef index():\n    return jsonify({'message': 'OK', 'status': 200})\n"
    );

    // run.py
    write_file_safe(&format!("{}/run.py", path),
        "from app import create_app\n\napp = create_app()\n\nif __name__ == '__main__':\n    app.run(debug=True)\n"
    );

    // requirements.txt
    write_file_safe(&format!("{}/requirements.txt", path),
        "Flask>=3.0\nFlask-SQLAlchemy>=3.1\nFlask-Migrate>=4.0\npython-decouple>=3.8\ngunicorn>=21.0\n"
    );

    write_file_safe(&format!("{}/.gitignore", path),
        "__pycache__/\n*.pyc\n.env\nvenv/\n*.db\ninstance/\n"
    );
    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\nFlask 웹 애플리케이션\n\n## 설치\n\n```bash\npip install -r requirements.txt\n```\n\n## 실행\n\n```bash\npython run.py\n```\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ Flask 프로젝트 생성: {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── FastAPI ──────────────────────────────────────────────────────────────────

fn scaffold_fastapi(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(&format!("{}/app", path))?;
    std::fs::create_dir_all(&format!("{}/app/api", path))?;
    std::fs::create_dir_all(&format!("{}/app/models", path))?;
    std::fs::create_dir_all(&format!("{}/app/schemas", path))?;
    std::fs::create_dir_all(&format!("{}/app/services", path))?;
    std::fs::create_dir_all(&format!("{}/tests", path))?;

    // main.py
    write_file_safe(&format!("{}/main.py", path),
        "from fastapi import FastAPI\nfrom fastapi.middleware.cors import CORSMiddleware\nfrom app.api.router import api_router\n\napp = FastAPI(title=\"API\", version=\"0.1.0\")\n\napp.add_middleware(\n    CORSMiddleware,\n    allow_origins=[\"*\"],\n    allow_methods=[\"*\"],\n    allow_headers=[\"*\"],\n)\n\napp.include_router(api_router, prefix=\"/api/v1\")\n\n\n@app.get(\"/health\")\ndef health():\n    return {\"status\": \"ok\"}\n"
    );

    // app/api/router.py
    write_file_safe(&format!("{}/app/api/router.py", path),
        "from fastapi import APIRouter\nfrom app.api.endpoints import items\n\napi_router = APIRouter()\napi_router.include_router(items.router, prefix=\"/items\", tags=[\"items\"])\n"
    );

    write_file_safe(&format!("{}/app/api/__init__.py", path), "");
    write_file_safe(&format!("{}/app/__init__.py", path), "");

    // app/api/endpoints/items.py
    std::fs::create_dir_all(&format!("{}/app/api/endpoints", path))?;
    write_file_safe(&format!("{}/app/api/endpoints/__init__.py", path), "");
    write_file_safe(&format!("{}/app/api/endpoints/items.py", path),
        "from fastapi import APIRouter\n\nrouter = APIRouter()\n\n\n@router.get(\"/\")\ndef list_items():\n    return [{\"id\": 1, \"name\": \"Example\"}]\n"
    );

    // requirements.txt
    write_file_safe(&format!("{}/requirements.txt", path),
        "fastapi>=0.110\nuvicorn[standard]>=0.27\npydantic>=2.0\nsqlalchemy>=2.0\nalembic>=1.13\nhttpx>=0.27\npython-decouple>=3.8\n"
    );

    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\nFastAPI 백엔드\n\n## 설치\n\n```bash\npip install -r requirements.txt\n```\n\n## 실행\n\n```bash\nuvicorn main:app --reload\n```\n\n## API 문서\n\n- Swagger: http://localhost:8000/docs\n- ReDoc: http://localhost:8000/redoc\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ FastAPI 프로젝트 생성: {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Go ──────────────────────────────────────────────────────────────────────

fn scaffold_go(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(&format!("{}/cmd", path))?;
    std::fs::create_dir_all(&format!("{}/internal", path))?;
    std::fs::create_dir_all(&format!("{}/pkg", path))?;

    let module_name = format!("github.com/user/{}", name);

    // go mod init
    let _ = run_cmd_in(&["go", "mod", "init", &module_name], path);

    // cmd/main.go
    write_file_safe(&format!("{}/cmd/main.go", path), &format!(
        "package main\n\nimport \"fmt\"\n\nfunc main() {{\n\tfmt.Println(\"Hello, {}!\")\n}}\n",
        name
    ));

    // Makefile
    write_file_safe(&format!("{}/Makefile", path),
        "build:\n\tgo build -o bin/app ./cmd/\n\nrun:\n\tgo run ./cmd/\n\ntest:\n\tgo test ./...\n\nlint:\n\tgolangci-lint run\n\n.PHONY: build run test lint\n"
    );

    write_file_safe(&format!("{}/.gitignore", path),
        "bin/\n*.exe\n*.test\n.env\nvendor/\n"
    );

    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\nGo 프로젝트\n\n## 빌드\n\n```bash\nmake build\n```\n\n## 실행\n\n```bash\nmake run\n```\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ Go 프로젝트 생성: {}", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Express.js ──────────────────────────────────────────────────────────────

fn scaffold_express(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(&format!("{}/src", path))?;
    std::fs::create_dir_all(&format!("{}/src/routes", path))?;
    std::fs::create_dir_all(&format!("{}/src/middleware", path))?;
    std::fs::create_dir_all(&format!("{}/src/controllers", path))?;

    write_file_safe(&format!("{}/package.json", path), &format!(
        "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0\",\n  \"scripts\": {{\n    \"start\": \"node src/app.js\",\n    \"dev\": \"nodemon src/app.js\",\n    \"test\": \"jest\"\n  }},\n  \"dependencies\": {{\n    \"express\": \"^4.18.0\",\n    \"cors\": \"^2.8.5\",\n    \"helmet\": \"^7.0.0\",\n    \"dotenv\": \"^16.0.0\"\n  }},\n  \"devDependencies\": {{\n    \"nodemon\": \"^3.0.0\",\n    \"jest\": \"^29.0.0\"\n  }}\n}}\n",
        name
    ));

    write_file_safe(&format!("{}/src/app.js", path),
        "const express = require('express');\nconst cors = require('cors');\nconst helmet = require('helmet');\nrequire('dotenv').config();\n\nconst app = express();\nconst PORT = process.env.PORT || 3000;\n\napp.use(helmet());\napp.use(cors());\napp.use(express.json());\n\napp.get('/health', (req, res) => res.json({ status: 'ok' }));\n\nconst itemsRouter = require('./routes/items');\napp.use('/api/items', itemsRouter);\n\napp.listen(PORT, () => console.log(`Server running on port ${PORT}`));\n\nmodule.exports = app;\n"
    );

    write_file_safe(&format!("{}/src/routes/items.js", path),
        "const express = require('express');\nconst router = express.Router();\n\nrouter.get('/', (req, res) => {\n  res.json([{ id: 1, name: 'Example' }]);\n});\n\nmodule.exports = router;\n"
    );

    write_file_safe(&format!("{}/.env.example", path),
        "PORT=3000\nNODE_ENV=development\n"
    );

    write_file_safe(&format!("{}/.gitignore", path),
        "node_modules/\n.env\n*.log\ncoverage/\n"
    );

    write_file_safe(&format!("{}/README.md", path), &format!(
        "# {}\n\nExpress.js API\n\n## 설치\n\n```bash\nnpm install\n```\n\n## 실행\n\n```bash\nnpm run dev\n```\n",
        name
    ));

    Ok(ScaffoldResult {
        output: format!("✅ Express.js 프로젝트 생성: {}\n(npm install 실행 필요)", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── C++ ─────────────────────────────────────────────────────────────────────

fn scaffold_cpp(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(&format!("{}/src", path))?;
    std::fs::create_dir_all(&format!("{}/include", path))?;
    std::fs::create_dir_all(&format!("{}/tests", path))?;
    std::fs::create_dir_all(&format!("{}/build", path))?;

    write_file_safe(&format!("{}/src/main.cpp", path), &format!(
        "#include <iostream>\n#include \"{}.h\"\n\nint main() {{\n    std::cout << \"Hello, {}!\" << std::endl;\n    return 0;\n}}\n",
        name, name
    ));

    write_file_safe(&format!("{}/include/{}.h", path, name), &format!(
        "#pragma once\n\n// {} header\n",
        name
    ));

    write_file_safe(&format!("{}/CMakeLists.txt", path), &format!(
        "cmake_minimum_required(VERSION 3.20)\nproject({} VERSION 0.1.0)\n\nset(CMAKE_CXX_STANDARD 17)\nset(CMAKE_CXX_STANDARD_REQUIRED True)\n\ninclude_directories(include)\n\nadd_executable({} src/main.cpp)\n",
        name, name
    ));

    write_file_safe(&format!("{}/Makefile", path), &format!(
        "build:\n\tcmake -B build && cmake --build build\n\nrun:\n\t./build/{}\n\nclean:\n\trm -rf build/\n\n.PHONY: build run clean\n",
        name
    ));

    write_file_safe(&format!("{}/.gitignore", path),
        "build/\n*.o\n*.a\n*.so\n*.exe\n"
    );

    Ok(ScaffoldResult {
        output: format!("✅ C++ 프로젝트 생성: {}\n빌드: make build", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── Deno ────────────────────────────────────────────────────────────────────

fn scaffold_deno(name: &str, path: &str) -> Result<ScaffoldResult> {
    std::fs::create_dir_all(path)?;

    write_file_safe(&format!("{}/main.ts", path), &format!(
        "// {} - Deno application\n\nconsole.log('Hello, {}!');\n",
        name, name
    ));

    write_file_safe(&format!("{}/deno.json", path), &format!(
        "{{\n  \"tasks\": {{\n    \"start\": \"deno run main.ts\",\n    \"dev\": \"deno run --watch main.ts\",\n    \"test\": \"deno test\"\n  }},\n  \"imports\": {{\n  }}\n}}\n"
    ));

    Ok(ScaffoldResult {
        output: format!("✅ Deno 프로젝트 생성: {}\n실행: deno task start", path),
        success: true,
        path: path.to_string(),
    })
}

// ─── CI/CD 설정 생성 ─────────────────────────────────────────────────────────

/// GitHub Actions 워크플로우 생성
pub fn generate_github_actions(project_type: &str, path: &str) -> Result<String> {
    std::fs::create_dir_all(&format!("{}/.github/workflows", path))?;

    let ci_content = match project_type.to_lowercase().as_str() {
        "rust" => RUST_CI,
        "python" | "django" | "flask" | "fastapi" => PYTHON_CI,
        "node" | "typescript" | "react" | "vue" | "next" | "express" => NODE_CI,
        "go" | "golang" => GO_CI,
        _ => GENERIC_CI,
    };

    write_file_safe(&format!("{}/.github/workflows/ci.yml", path), ci_content);
    Ok(format!("✅ GitHub Actions CI 생성: {}/.github/workflows/ci.yml", path))
}

/// PR 템플릿 생성
pub fn generate_pr_template(path: &str) -> Result<String> {
    std::fs::create_dir_all(&format!("{}/.github", path))?;

    write_file_safe(&format!("{}/.github/PULL_REQUEST_TEMPLATE.md", path),
        "## 변경 사항\n\n<!-- 무엇을 왜 변경했는지 설명 -->\n\n## 변경 유형\n\n- [ ] feat: 새 기능\n- [ ] fix: 버그 수정\n- [ ] refactor: 리팩토링\n- [ ] docs: 문서\n- [ ] test: 테스트\n- [ ] chore: 기타\n\n## 체크리스트\n\n- [ ] 코드 리뷰 요청\n- [ ] 테스트 추가/수정\n- [ ] 문서 업데이트\n- [ ] Breaking change 없음\n\n## 스크린샷 (UI 변경 시)\n\n"
    );

    write_file_safe(&format!("{}/.github/ISSUE_TEMPLATE/bug_report.md", path),
        "---\nname: 버그 리포트\nabout: 버그를 신고해주세요\ntitle: '[BUG] '\nlabels: bug\n---\n\n## 버그 설명\n\n## 재현 방법\n\n## 기대 동작\n\n## 실제 동작\n\n## 환경\n- OS: \n- 버전: \n"
    );

    std::fs::create_dir_all(&format!("{}/.github/ISSUE_TEMPLATE", path))?;

    Ok(format!("✅ PR/이슈 템플릿 생성: {}/.github/", path))
}

// ─── CI/CD 템플릿 ─────────────────────────────────────────────────────────────

const RUST_CI: &str = r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Check formatting
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Test
        run: cargo test
      - name: Build
        run: cargo build --release
"#;

const PYTHON_CI: &str = r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python-version: ["3.11", "3.12"]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}
      - name: Install dependencies
        run: |
          pip install -r requirements.txt
          pip install pytest pytest-cov flake8 black mypy
      - name: Lint
        run: |
          black --check .
          flake8 . --max-line-length=100
      - name: Test
        run: pytest --cov=. --cov-report=xml
"#;

const NODE_CI: &str = r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node-version: [18.x, 20.x]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ matrix.node-version }}
          cache: 'npm'
      - run: npm ci
      - run: npm run lint --if-present
      - run: npm test
      - run: npm run build --if-present
"#;

const GO_CI: &str = r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: '1.22'
      - name: Vet
        run: go vet ./...
      - name: Test
        run: go test ./...
      - name: Build
        run: go build ./...
"#;

const GENERIC_CI: &str = r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: echo "Add build steps here"
"#;

// ─── Helpers ─────────────────────────────────────────────────────────────────────

fn run_cmd(args: &[&str]) -> Result<String> {
    let timeout = Duration::from_secs(SCAFFOLD_TIMEOUT);
    let program = args[0].to_string();
    let rest: Vec<String> = args[1..].iter().map(|s| s.to_string()).collect();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new(&program).args(&rest).output();
        let _ = tx.send(out);
    });

    let output = rx.recv_timeout(timeout)
        .context("타임아웃")?
        .with_context(|| format!("실행 실패: {}", args[0]))?;

    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(out.trim().to_string())
    } else {
        anyhow::bail!("{}", out.trim())
    }
}

fn run_cmd_in(args: &[&str], cwd: &str) -> Result<String> {
    let timeout = Duration::from_secs(SCAFFOLD_TIMEOUT);
    let program = args[0].to_string();
    let rest: Vec<String> = args[1..].iter().map(|s| s.to_string()).collect();
    let cwd_owned = cwd.to_string();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new(&program).args(&rest).current_dir(&cwd_owned).output();
        let _ = tx.send(out);
    });

    let output = rx.recv_timeout(timeout)
        .context("타임아웃")?
        .with_context(|| format!("실행 실패: {}", args[0]))?;

    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        Ok(out.trim().to_string())
    } else {
        anyhow::bail!("{}", out.trim())
    }
}

fn write_file_safe(path: &str, content: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, content);
}
