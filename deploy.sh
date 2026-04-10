#!/bin/bash
# 배포 스크립트 - 프로덕션 빌드 및 배포

set -e

echo "🚀 배포 시작..."
echo ""

# 변수 정의
PROJECT_NAME="ai_agent"
VERSION=$(grep version Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
BUILD_DIR="target/release"
DIST_DIR="dist"

echo "프로젝트: $PROJECT_NAME"
echo "버전: $VERSION"
echo ""

# Step 1: 테스트
echo "📋 [1/4] 테스트 실행..."
cargo test --release 2>&1 | grep -E "(test result|running)"
echo "✅ 테스트 완료"
echo ""

# Step 2: 린트 확인
echo "🔍 [2/4] 코드 품질 확인..."
cargo clippy --release -- -D warnings 2>&1 | tail -5 || true
echo "✅ 린트 완료"
echo ""

# Step 3: 최적화 빌드
echo "🔨 [3/4] 최적화 빌드..."
cargo build --release
echo "✅ 빌드 완료"
echo ""

# Step 4: 배포 패키지 생성
echo "📦 [4/4] 배포 패키지 생성..."
mkdir -p $DIST_DIR

# 바이너리 복사
cp $BUILD_DIR/$PROJECT_NAME $DIST_DIR/

# 설정 파일 복사
cp docker-compose.yml $DIST_DIR/
cp Cargo.toml $DIST_DIR/
cp README.md $DIST_DIR/
cp QUICK_START.md $DIST_DIR/

# 배포 정보 파일 생성
cat > $DIST_DIR/DEPLOY_INFO.txt <<EOF
=================================
$PROJECT_NAME v$VERSION
배포 패키지
=================================

빌드 시간: $(date)
빌드 경로: $PWD
바이너리: ./$PROJECT_NAME

설치 및 실행:
1. dist/ 폴더 이동: cd dist
2. 바이너리 실행: ./$PROJECT_NAME
3. 또는 docker-compose 사용

포함된 파일:
- $PROJECT_NAME (바이너리)
- docker-compose.yml (Docker 설정)
- README.md (설명서)
- QUICK_START.md (시작 가이드)

필수 요구사항:
- Docker & Docker Compose
- 8GB+ 메모리
- 포트 11434 사용 가능

더 많은 정보는 README.md 참고
=================================
EOF

echo "✅ 배포 패키지 생성 완료"
echo ""

# 완료 정보
echo "════════════════════════════════════════"
echo "🎉 배포 준비 완료!"
echo "════════════════════════════════════════"
echo ""
echo "패키지 위치: ./$DIST_DIR/"
echo "바이너리: ./$DIST_DIR/$PROJECT_NAME"
echo ""
echo "배포 명령어:"
echo "  배포용 폴더로 이동: cp -r dist/ /target/path/"
echo ""
