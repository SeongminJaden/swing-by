/// Docker / 컨테이너 관리 툴

use anyhow::Result;
use std::process::Command;
use std::time::Duration;

const DOCKER_TIMEOUT: u64 = 300;
const MAX_OUTPUT: usize = 16_000;

#[derive(Debug)]
pub struct DockerResult {
    pub output: String,
    #[allow(dead_code)]
    pub success: bool,
}

impl std::fmt::Display for DockerResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
    }
}

fn run_docker(args: &[&str], cwd: Option<&str>) -> Result<DockerResult> {
    run_docker_with_timeout(args, cwd, DOCKER_TIMEOUT)
}

fn run_docker_with_timeout(args: &[&str], cwd: Option<&str>, timeout_secs: u64) -> Result<DockerResult> {
    if !is_docker_available() {
        anyhow::bail!("Docker가 설치되지 않았거나 실행 중이 아닙니다.\n설치: https://docs.docker.com/engine/install/");
    }

    let timeout = Duration::from_secs(timeout_secs);
    let program = "docker".to_string();
    let rest: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd_owned = cwd.map(|s| s.to_string());

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut cmd = Command::new(&program);
        cmd.args(&rest);
        if let Some(ref d) = cwd_owned { cmd.current_dir(d); }
        let _ = tx.send(cmd.output());
    });

    let output = rx.recv_timeout(timeout)
        .map_err(|_| anyhow::anyhow!("Docker 타임아웃 ({}초)", timeout_secs))?
        .map_err(|e| anyhow::anyhow!("Docker 실행 실패: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    let combined = if success {
        if stdout.is_empty() { stderr } else { stdout }
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let out_text = if combined.len() > MAX_OUTPUT {
        format!("{}...[잘림]", crate::utils::trunc(&combined, MAX_OUTPUT))
    } else {
        combined.trim().to_string()
    };

    Ok(DockerResult { output: out_text, success })
}

fn is_docker_available() -> bool {
    Command::new("docker").arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ─── Docker 명령어 ────────────────────────────────────────────────────────────

/// docker ps [all]
pub fn docker_ps(all: bool) -> Result<DockerResult> {
    if all {
        run_docker(&["ps", "-a", "--format", "table {{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}"], None)
    } else {
        run_docker(&["ps", "--format", "table {{.ID}}\t{{.Image}}\t{{.Status}}\t{{.Names}}"], None)
    }
}

/// docker images
pub fn docker_images() -> Result<DockerResult> {
    run_docker(&["images", "--format", "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"], None)
}

/// docker pull <image>
pub fn docker_pull(image: &str) -> Result<DockerResult> {
    run_docker_with_timeout(&["pull", image], None, 120)
}

/// docker build -t <tag> [context]
pub fn docker_build(tag: &str, context: &str, dockerfile: &str) -> Result<DockerResult> {
    let ctx = if context.is_empty() { "." } else { context };
    if dockerfile.is_empty() {
        run_docker_with_timeout(&["build", "-t", tag, ctx], Some(ctx), DOCKER_TIMEOUT)
    } else {
        run_docker_with_timeout(&["build", "-t", tag, "-f", dockerfile, ctx], Some(ctx), DOCKER_TIMEOUT)
    }
}

/// docker run [options] <image> [cmd]
pub fn docker_run(image: &str, options: &str, cmd: &str) -> Result<DockerResult> {
    let opts: Vec<&str> = if options.is_empty() { vec![] } else { options.split_whitespace().collect() };
    let mut full_args: Vec<&str> = vec!["run", "--rm"];
    full_args.extend(opts.iter());
    full_args.push(image);
    if !cmd.is_empty() {
        full_args.extend(cmd.split_whitespace());
    }
    run_docker_with_timeout(&full_args, None, 60)
}

/// docker stop/start/restart <container>
pub fn docker_control(action: &str, container: &str) -> Result<DockerResult> {
    let action = match action.to_lowercase().as_str() {
        "stop" | "start" | "restart" | "rm" | "kill" => action.to_lowercase(),
        other => anyhow::bail!("유효하지 않은 docker 액션: '{}'", other),
    };
    run_docker(&[&action, container], None)
}

/// docker logs <container> [tail=50]
pub fn docker_logs(container: &str, tail: usize) -> Result<DockerResult> {
    let tail_str = tail.to_string();
    run_docker(&["logs", "--tail", &tail_str, container], None)
}

/// docker exec <container> <cmd>
pub fn docker_exec(container: &str, cmd: &str) -> Result<DockerResult> {
    let mut args = vec!["exec", container];
    args.extend(cmd.split_whitespace());
    run_docker(&args, None)
}

/// docker inspect <container|image>
pub fn docker_inspect(target: &str) -> Result<DockerResult> {
    run_docker(&["inspect", target], None)
}

/// docker stats (snapshot, no stream)
pub fn docker_stats() -> Result<DockerResult> {
    run_docker(&["stats", "--no-stream",
        "--format", "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}"],
        None)
}

/// docker network ls
pub fn docker_network_ls() -> Result<DockerResult> {
    run_docker(&["network", "ls", "--format",
        "table {{.ID}}\t{{.Name}}\t{{.Driver}}\t{{.Scope}}"], None)
}

/// docker network inspect <network>
pub fn docker_network_inspect(network: &str) -> Result<DockerResult> {
    run_docker(&["network", "inspect", network], None)
}

/// docker volume ls
pub fn docker_volume_ls() -> Result<DockerResult> {
    run_docker(&["volume", "ls", "--format",
        "table {{.Name}}\t{{.Driver}}\t{{.Mountpoint}}"], None)
}

/// docker volume rm <volume>
pub fn docker_volume_rm(volume: &str) -> Result<DockerResult> {
    run_docker(&["volume", "rm", volume], None)
}

/// docker system prune (unused objects 정리)
pub fn docker_prune(all: bool) -> Result<DockerResult> {
    if all {
        run_docker_with_timeout(&["system", "prune", "-af"], None, 120)
    } else {
        run_docker_with_timeout(&["system", "prune", "-f"], None, 60)
    }
}

/// docker compose up/down/build
pub fn docker_compose(action: &str, path: &str, detach: bool) -> Result<DockerResult> {
    let cwd = if path.is_empty() { "." } else { path };
    let action_str = action.to_lowercase();

    // docker compose (v2) 또는 docker-compose (v1)
    let compose_cmd = if docker_compose_v2_available() { "compose" } else { "" };

    let result = match action_str.as_str() {
        "up" => {
            if compose_cmd.is_empty() {
                // docker-compose 시도
                run_compose_legacy(&["up", if detach { "-d" } else { "" }].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>(), cwd)
            } else if detach {
                run_docker(&["compose", "up", "-d"], Some(cwd))
            } else {
                run_docker_with_timeout(&["compose", "up"], Some(cwd), 120)
            }
        }
        "down" => {
            if compose_cmd.is_empty() {
                run_compose_legacy(&["down"], cwd)
            } else {
                run_docker(&["compose", "down"], Some(cwd))
            }
        }
        "build" => {
            if compose_cmd.is_empty() {
                run_compose_legacy(&["build"], cwd)
            } else {
                run_docker_with_timeout(&["compose", "build"], Some(cwd), DOCKER_TIMEOUT)
            }
        }
        "ps" => {
            if compose_cmd.is_empty() {
                run_compose_legacy(&["ps"], cwd)
            } else {
                run_docker(&["compose", "ps"], Some(cwd))
            }
        }
        "logs" => {
            if compose_cmd.is_empty() {
                run_compose_legacy(&["logs", "--tail=50"], cwd)
            } else {
                run_docker(&["compose", "logs", "--tail=50"], Some(cwd))
            }
        }
        other => anyhow::bail!("유효하지 않은 compose 액션: '{}'", other),
    };

    result
}

fn docker_compose_v2_available() -> bool {
    Command::new("docker").args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_compose_legacy(args: &[&str], cwd: &str) -> Result<DockerResult> {
    let timeout = Duration::from_secs(120);
    let rest: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd_owned = cwd.to_string();

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new("docker-compose")
            .args(&rest)
            .current_dir(&cwd_owned)
            .output();
        let _ = tx.send(out);
    });

    let output = rx.recv_timeout(timeout)
        .map_err(|_| anyhow::anyhow!("docker-compose 타임아웃"))?
        .map_err(|e| anyhow::anyhow!("docker-compose 실행 실패: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = if stdout.is_empty() { stderr } else { stdout };

    Ok(DockerResult {
        output: combined.trim().to_string(),
        success: output.status.success(),
    })
}

/// Dockerfile 자동 생성
pub fn generate_dockerfile(language: &str, project_name: &str, path: &str) -> Result<String> {
    let content = match language.to_lowercase().as_str() {
        "python" | "django" | "flask" | "fastapi" => dockerfile_python(project_name),
        "node" | "javascript" | "express" => dockerfile_node(project_name),
        "typescript" | "next" | "react" => dockerfile_node_ts(project_name),
        "rust" => dockerfile_rust(project_name),
        "go" | "golang" => dockerfile_go(project_name),
        "java" | "spring" => dockerfile_java(project_name),
        other => anyhow::bail!("지원하지 않는 Dockerfile 생성 언어: '{}'", other),
    };

    let target = if path.is_empty() { "Dockerfile".to_string() }
        else { format!("{}/Dockerfile", path.trim_end_matches('/')) };

    if let Some(parent) = std::path::Path::new(&target).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&target, &content)
        .map_err(|e| anyhow::anyhow!("Dockerfile 저장 실패: {}", e))?;

    // docker-compose.yml도 생성
    let compose_path = if path.is_empty() { "docker-compose.yml".to_string() }
        else { format!("{}/docker-compose.yml", path) };

    let compose_content = generate_compose(language, project_name);
    let _ = std::fs::write(&compose_path, &compose_content);

    Ok(format!("✅ Dockerfile + docker-compose.yml 생성: {}", target))
}

fn dockerfile_python(name: &str) -> String {
    format!(r#"FROM python:3.12-slim

WORKDIR /app

# 의존성 먼저 설치 (캐시 활용)
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]

# 빌드: docker build -t {} .
# 실행: docker run -p 8000:8000 {}
"#, name, name)
}

fn dockerfile_node(name: &str) -> String {
    format!(r#"FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

EXPOSE 3000

CMD ["node", "src/app.js"]

# 빌드: docker build -t {} .
# 실행: docker run -p 3000:3000 {}
"#, name, name)
}

fn dockerfile_node_ts(name: &str) -> String {
    format!(r#"FROM node:20-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:20-alpine AS runner
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/package*.json ./
RUN npm ci --only=production

EXPOSE 3000
CMD ["node", "dist/index.js"]

# 멀티스테이지 빌드: {} 프로덕션 이미지
"#, name)
}

fn dockerfile_rust(name: &str) -> String {
    format!(r#"FROM rust:1.80-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {{}}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/{} /usr/local/bin/app

EXPOSE 8080
CMD ["app"]
"#, name)
}

fn dockerfile_go(name: &str) -> String {
    format!(r#"FROM golang:1.22-alpine AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o {} ./cmd/

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /app/{} .

EXPOSE 8080
CMD ["./{}"]
"#, name, name, name)
}

fn dockerfile_java(name: &str) -> String {
    format!(r#"FROM maven:3.9-eclipse-temurin-21 AS builder

WORKDIR /app
COPY pom.xml .
RUN mvn dependency:go-offline -q
COPY src ./src
RUN mvn package -q -DskipTests

FROM eclipse-temurin:21-jre-alpine
WORKDIR /app
COPY --from=builder /app/target/*.jar app.jar

EXPOSE 8080
CMD ["java", "-jar", "app.jar"]

# {} Spring Boot 애플리케이션
"#, name)
}

fn generate_compose(language: &str, name: &str) -> String {
    let port = match language {
        "rust" | "go" | "java" | "spring" => "8080",
        "react" | "next" => "3000",
        _ => "8000",
    };

    format!(r#"version: '3.8'

services:
  app:
    build: .
    container_name: {name}
    ports:
      - "{port}:{port}"
    environment:
      - NODE_ENV=production
    restart: unless-stopped
    volumes:
      - .:/app
      - /app/node_modules

  # 데이터베이스 (필요 시 주석 해제)
  # db:
  #   image: postgres:16-alpine
  #   environment:
  #     POSTGRES_DB: {name}
  #     POSTGRES_USER: user
  #     POSTGRES_PASSWORD: password
  #   volumes:
  #     - postgres_data:/var/lib/postgresql/data
  #   ports:
  #     - "5432:5432"

# volumes:
#   postgres_data:
"#, name=name, port=port)
}
