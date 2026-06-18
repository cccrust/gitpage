# Gitpage 測試計畫

## 概述

本文檔定義 Gitpage 專案的多層次測試策略，從最小的單元測試到完整的端到端測試，覆蓋後端 Rust、前端 React、Git 協定、CLI 操作等所有面向。

## 測試金字塔

```
            ╱╲
           ╱  ╲  E2E (Playwright 6 scenarios)
          ╱────╲
         ╱      ╲  CLI (bash + assert 5 scenarios)
        ╱────────╲
       ╱          ╲  Integration (bash+curl 14 scripts 120+ steps)
      ╱────────────╲
     ╱              ╲  REST API (hurl 30+ endpoint scenarios)
    ╱────────────────╲
   ╱                  ╲  Unit Tests (#[cfg(test)] 15+ modules 100+ tests)
  ╱────────────────────╲
```

## 層級 1：單元測試（Unit Tests）

### 當前狀態

Rust 程式碼中完全沒有任何 `#[cfg(test)]` 模組。這是最大的測試缺口。

### 目標

為每個 Rust 模組新增 `#[cfg(test)] mod tests`，覆蓋核心邏輯路徑。

### 測試模組配置

| 模組 | 測試覆蓋重點 | 預計測試數 |
|------|-------------|-----------|
| `auth/mod.rs` | create_token 成功/驗證/到期/無效簽章/OnceLock 行為；encrypt_secret/decrypt_secret 往返/nonce 唯一性 | 14 |
| `config.rs` | TOML 解析、預設值、環境變數覆蓋、repo_path/staging_path/app_workspace_dir/pages_dir 正確性 | 8 |
| `db/mod.rs` | CRUD（使用者/倉庫/組織/Issue/PR）、唯一性約束、外部鍵、部分索引 | 15 |
| `db/models.rs` | JSON 序列化、UserPublic 不洩漏 password_hash | 5 |
| `git/mod.rs` | build_tree_from_dir 遞迴（mock 檔案系統）、init_bare_repo、repo_exists | 6 |
| `deploy.rs` | detect_project_type、resolve_commands 覆蓋邏輯、allocate_port 範圍掃描 | 8 |
| `docker.rs` | generate_random_password 長度/字元集、SSH port 範圍、容器名稱格式 | 5 |
| `ssh.rs` | regenerate_authorized_keys 格式、限制選項包含 | 4 |
| `handlers/auth.rs` | 短密碼拒絕、重複使用者名、argon2 hash 格式 | 6 |
| `handlers/repos.rs` | resolve_repo 解析順序、私有倉庫權限 | 6 |
| `handlers/files.rs` | safe_path 防護（`..`、`/` 前綴） | 4 |
| `handlers/settings.rs` | AES 加解密往返、access token 格式（`gpt_` prefix） | 5 |
| `handlers/pulls.rs` | merge_pr 三方合併（無衝突/有衝突） | 4 |
| `utils/errors.rs` | 各變體 HTTP 狀態碼、From 轉換 | 8 |
| 其他 handler | 輸入驗證、邊界條件 | 12 |

**總計：約 110 個單元測試**

### 測試工具與慣例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.run_migrations().unwrap();
        db
    }

    fn setup_test_secrets() {
        init_jwt_secret("test-secret");
        init_encryption_key("test-key");
    }
}
```

### 執行方式

```bash
cargo test                               # 全部
cargo test test_auth                     # 特定模組
cargo test test_config_repo_path -- --nocapture  # 含輸出
cargo test -- --skip test_docker          # 跳過需 Docker 的測試
```

## 層級 2：REST API 測試

### 當前狀態

`test.sh` 等腳本涵蓋基本 API 路徑，但手動解析 JSON、非結構化、難以維護。

### 目標

使用 [hurl](https://hurl.dev/) 建立結構化的 API 測試，每個端點有明確的請求和斷言。

### 為什麼選 hurl

| 特性 | hurl | bash+curl | Postman/Newman |
|------|------|-----------|----------------|
| 宣告式語法 | ✅ | ❌ | ✅ |
| JSON 斷言內建 | ✅ | ❌ 需 python3 | ✅ |
| 變數共享 | ✅ | ✅ | ✅ |
| 依賴安裝 | ❌ 需安裝 | ✅ 內建 | ❌ Node.js |
| CI 整合 | ✅ | ✅ | ✅ |

### 測試檔案結構

```
tests/
├── api/
│   ├── auth.hurl            # 註冊/登入/Me/密碼變更
│   ├── repos.hurl           # CRUD/搜尋/Fork
│   ├── content.hurl         # Tree/Blob/Readme/Commits
│   ├── files.hurl           # Staging 檔案操作
│   ├── pages.hurl           # Pages 設定與部署
│   ├── apps.hurl            # Apps 設定與部署
│   ├── orgs.hurl            # 組織 CRUD 與成員
│   ├── issues.hurl          # Issue CRUD/標籤/評論
│   ├── pulls.hurl           # PR CRUD/Diff/Merge
│   ├── settings.hurl        # Token/協作者/Secrets/分支保護
│   ├── stars.hurl           # Star/Watch
│   └── errors.hurl          # 錯誤案例（400/401/404/409）
└── run_api_tests.sh
```

### 範例（auth.hurl）

```hurl
POST {{base_url}}/api/auth/register
Content-Type: application/json
{
    "username": "hurl-test-{{timestamp}}",
    "email": "hurl@test.com",
    "password": "pass123"
}
HTTP 201
[Asserts]
jsonpath "$.token" isString
jsonpath "$.user.id" exists

POST {{base_url}}/api/auth/register
Content-Type: application/json
{
    "username": "hurl-test-{{timestamp}}",
    "email": "hurl@test.com",
    "password": "pass123"
}
HTTP 409
[Asserts]
jsonpath "$.error" contains "已存在"

POST {{base_url}}/api/auth/login
Content-Type: application/json
{
    "username": "hurl-test-{{timestamp}}",
    "password": "pass123"
}
HTTP 200
[Asserts]
jsonpath "$.token" isString
[Captures]
auth_token = jsonpath "$.token"

GET {{base_url}}/api/auth/me
HTTP 401

GET {{base_url}}/api/auth/me
Authorization: Bearer {{auth_token}}
HTTP 200
[Asserts]
jsonpath "$.user.id" exists
```

### 執行方式

```bash
brew install hurl
cargo run &
hurl --variable base_url=http://localhost:8080 \
     --variable timestamp=$(date +%s) \
     tests/api/*.hurl
```

## 層級 3：整合測試（Integration Tests）

### 當前狀態

三個 `test_*.sh` 覆蓋 41/23/29 步，但缺 File Manager、App 部署、Issue/PR/Star/Watch、Settings、SSH 金鑰、錯誤路徑。

### 目標

重構成 14 個聚焦子腳本 + 共用函數庫。

### 新結構

```
test/
├── run_all.sh                # 依序執行所有子腳本
├── lib.sh                    # 共用函數（register_user/create_repo/git_push/assert_eq）
├── 01-auth.sh                # 註冊/登入/Me/密碼變更/重複註冊
├── 02-repo.sh                # 倉庫 CRUD/搜尋/Fork/私人倉庫權限
├── 03-content.sh             # Tree/Blob/Readme/Commit/markdown 渲染
├── 04-files.sh               # Staging 寫入/刪除/移動/狀態/提交/路徑穿越防護
├── 05-pages.sh               # Pages 設定/部署/重新部署
├── 06-app.sh                 # Apps 設定/建置/啟動/部署記錄/健康檢查
├── 07-org.sh                 # 組織 CRUD/成員管理/組織倉庫
├── 08-issues.sh              # Issue CRUD/標籤/評論
├── 09-pulls.sh               # PR 建立/合併/Diff
├── 10-settings.sh            # Token/協作者/Secrets/分支保護
├── 11-stars.sh               # Star/Watch 切換/計數
├── 12-ssh-keys.sh            # SSH 金鑰新增/刪除/列表
├── 13-git-protocol.sh        # Git push/clone 完整流程
├── 14-error-paths.sh         # 401/404/409/400 錯誤案例
└── docker/
    ├── docker-compose-test.sh    # Docker 內整合測試
    └── docker-mode-test.sh       # Docker runtime 模式測試
```

### 共用函數庫（lib.sh）

```bash
BASE="http://localhost:8080"
TIMESTAMP=$(date +%s)

api() {
    curl -s -X "$1" "$BASE$2" \
        ${3:+-H "Authorization: Bearer $3"} \
        ${4:+-H "Content-Type: application/json" -d "$4"}
}

register_user() {
    local u=$1
    api POST "/api/auth/register" "" \
        "{\"username\":\"$u-$TIMESTAMP\",\"email\":\"$u@test.com\",\"password\":\"pass123\"}" \
        | python3 -c "import sys,json;print(json.load(sys.stdin).get('token',''))"
}

create_repo() {
    local tk=$1 name=$2
    api POST "/api/repos" "$tk" \
        "{\"name\":\"$name\",\"description\":\"test\"}" \
        | python3 -c "import sys,json;print(json.load(sys.stdin).get('repo',{}).get('id',0))"
}

git_push() {
    local user=$1 repo=$2
    local dir="/tmp/gptest-$user-$repo"
    rm -rf "$dir" && mkdir -p "$dir"
    cd "$dir" || exit 1
    git init -q && git config user.email "$user@test.com" && git config user.name "$user"
    echo "# $repo" > README.md
    git add -A && git commit -q -m "init"
    git remote add origin "$BASE/git/$user/$repo"
    git push origin main 2>&1
    cd - > /dev/null
}

assert_eq()     { [ "$1" = "$2" ] || { echo "FAIL: '$1' != '$2' ($BASH_LINENO)"; exit 1; } }
assert_status() { local s; s=$(api GET "$1" "$2" -o /dev/null -w "%{http_code}"); [ "$s" = "${3:-200}" ] || { echo "FAIL: status $s != ${3:-200}"; exit 1; } }
assert_not_empty() { [ -n "$1" ] || { echo "FAIL: empty value ($BASH_LINENO)"; exit 1; } }
```

### 新領域測試範例

#### 04-files.sh（File Manager）

```bash
source test/lib.sh
echo "=== 04-files ==="
TK=$(register_user "files"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "filetest"); assert_not_empty "$RID"

# 寫入
api PUT "/api/repos/$RID/files?path=hello.txt" "$TK" "Hello, World!"
assert_status "/api/repos/$RID/files?path=hello.txt" "$TK" 200

# 列表
TREE=$(api GET "/api/repos/$RID/tree" "$TK")
echo "$TREE" | python3 -c "import sys,json;d=json.load(sys.stdin);assert len(d['entries'])==1"

# 子目錄 + 移動
api POST "/api/repos/$RID/mkdir?path=subdir" "$TK"
api PUT "/api/repos/$RID/files?path=subdir/data.txt" "$TK" "data"
api POST "/api/repos/$RID/move?from=hello.txt&to=hello2.txt" "$TK"

# 狀態 + 提交
api GET "/api/repos/$RID/status" "$TK" | python3 -m json.tool
api POST "/api/repos/$RID/commit" "$TK" '{"message":"test commit"}'

# 路徑穿越防護
api PUT "/api/repos/$RID/files?path=../../../etc/passwd" "$TK" "hack"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/repos/$RID/files?path=../../../etc/passwd" -H "Authorization: Bearer $TK" -d "hack")
assert_eq "$STATUS" "400"
echo "=== 04-files PASSED ==="
```

#### 10-settings.sh（Settings）

```bash
source test/lib.sh
echo "=== 10-settings ==="
TK=$(register_user "settings"); RID=$(create_repo "$TK" "settingstest")

# Access Token
TOKEN_RESP=$(api POST "/api/user/tokens" "$TK" '{"name":"t","scopes":["repo:read"]}')
RAW_TK=$(echo "$TOKEN_RESP" | python3 -c "import sys,json;print(json.load(sys.stdin).get('raw_token',''))")
[[ "$RAW_TK" = gpt_* ]] || { echo "FAIL: token prefix"; exit 1; }

# Collaborator
BOB_TK=$(register_user "bobset"); BOB_ID=$(api GET "/api/auth/me" "$BOB_TK" | python3 -c "import sys,json;print(json.load(sys.stdin)['user']['id'])")
api POST "/api/repos/$RID/collaborators" "$TK" "{\"username\":\"bobset-$TIMESTAMP\",\"permission\":\"read\"}"

# Secret
api POST "/api/repos/$RID/secrets" "$TK" '{"name":"DB_PASS","value":"supersecret"}'
SECRETS=$(api GET "/api/repos/$RID/secrets" "$TK")
VAL=$(echo "$SECRETS" | python3 -c "import sys,json;print(json.load(sys.stdin)['secrets'][0]['value'])")
assert_eq "$VAL" "supersecret"

# Branch Protection
api POST "/api/repos/$RID/branch-protections" "$TK" '{"pattern":"main","require_pull_request":true}'
echo "=== 10-settings PASSED ==="
```

### 執行方式

```bash
./test/run_all.sh        # 全部
./test/06-app.sh         # 單一領域
```

## 層級 4：CLI 測試

### 當前狀態

無任何 CLI 測試。

### 目標

測試 `gitpage` 二進位的命令列行為。

### 測試案例

| 案例 | 指令 | 預期 |
|------|------|------|
| `--help` | `./gitpage --help` | exit 0，包含 "Usage" |
| `--version` | `./gitpage --version` | exit 0，格式 `gitpage x.y.z` |
| 不存在設定檔 | `./gitpage nonexistent.toml` | exit 1，錯誤訊息 |
| 明確設定檔 | `./gitpage config.toml` | 啟動成功（timeout 5s 後 kill） |
| 無參數 | `./gitpage` | 使用預設 config.toml 啟動成功 |
| 環境變數覆蓋 | `JWT_SECRET=custom ./gitpage` | 啟動成功 |

### 實作（test/cli_test.sh）

```bash
#!/bin/bash
set -e
cargo build 2>&1 | tail -1
BIN="./target/debug/gitpage"

$BIN --help 2>&1 | grep -q "Usage" && echo "PASS: --help" || { echo "FAIL"; exit 1; }
$BIN --version 2>&1 | grep -qE "^gitpage [0-9]" && echo "PASS: --version" || { echo "FAIL"; exit 1; }
$BIN nonexistent.toml 2>&1 && { echo "FAIL: should error"; exit 1; } || echo "PASS: nonexistent config"

timeout 5 $BIN > /dev/null 2>&1 &
PID=$!; sleep 3; kill $PID 2>/dev/null; wait $PID 2>/dev/null; echo "PASS: startup"
```

## 層級 5：端到端測試（E2E Tests）

### 當前狀態

無任何 E2E 測試。

### 目標

使用 Playwright 模擬瀏覽器操作，覆蓋完整使用者流程。

### 為什麼選 Playwright

| 特性 | Playwright | Cypress | Puppeteer |
|------|-----------|---------|-----------|
| 語言 | TS/JS/Python/Java | JS | JS |
| 速度 | ✅ 快（CDP） | ❌ 慢（iframe） | ✅ 快 |
| 多瀏覽器 | ✅ Chromium/Firefox/Safari | ❌ Chromium only | ❌ Chromium only |
| 檔案上傳 | ✅ 內建 | ✅ | ✅ |

### 檔案結構

```
frontend/e2e/
├── playwright.config.ts
├── fixtures/auth.ts
└── specs/
    ├── auth.spec.ts          # 註冊/登入/登出完整流程
    ├── repo.spec.ts          # 建立/刪除/瀏覽倉庫
    ├── files.spec.ts         # 檔案管理器 UI 操作
    ├── pages.spec.ts         # Pages 設定與檢視
    ├── orgs.spec.ts          # 組織操作
    └── navigation.spec.ts    # 路由/導航正確性
```

### 關鍵場景

#### 場景 1：完整使用者旅程

```typescript
test('full user journey', async ({ page }) => {
    await page.goto('/');
    await page.click('text=註冊');
    await page.fill('[name=username]', 'e2e-test');
    await page.fill('[name=email]', 'e2e@test.com');
    await page.fill('[name=password]', 'pass123');
    await page.click('button:has-text("註冊")');
    await expect(page.locator('text=儀表板')).toBeVisible();

    await page.click('text=新增');
    await page.fill('[name=name]', 'e2e-repo');
    await page.click('button:has-text("建立")');
    await expect(page.locator('text=e2e-repo')).toBeVisible();

    await page.click('text=登出');
    await expect(page.locator('text=登入')).toBeVisible();
});
```

#### 場景 2：File Manager UI

```typescript
test('file manager', async ({ page }) => {
    await createRepoViaAPI(page); // 前置
    await page.goto('/repo/1/files');

    await page.click('text=新增檔案');
    await page.fill('[name=path]', 'test.txt');
    await page.fill('[name=content]', 'Hello');
    await page.click('button:has-text("儲存")');
    await expect(page.locator('text=test.txt')).toBeVisible();

    await page.click('text=提交');
    await page.fill('[name=message]', 'e2e commit');
    await page.click('button:has-text("確認提交")');
    await expect(page.locator('text=提交成功')).toBeVisible();
});
```

#### 場景 3：Git Push + Pages 部署

```typescript
test('git push and pages', async ({ page }) => {
    const { repoId, username } = await setupTestUser(page);
    await exec(`cd /tmp/e2e-test && git init && echo "test" > index.html
        && git add -A && git commit -m "init"
        && git remote add origin http://localhost:8080/git/${username}/e2e-pages
        && git push origin main`);

    await page.goto(`/repo/${repoId}/pages`);
    await page.check('input[name=enabled]');
    await page.click('button:has-text("儲存")');
    await expect(page.locator('text=部署成功')).toBeVisible();

    const resp = await page.request.get(`/pages/${username}/e2e-pages/`);
    expect(resp.status()).toBe(200);
});
```

#### 場景 4：Issue/PR 協作

```typescript
test('issue collaboration', async ({ page }) => {
    const { repoId } = await setupTestUser(page);
    await page.goto(`/repo/${repoId}/issues`);
    await page.click('text=新增 Issue');
    await page.fill('[name=title]', 'Bug report');
    await page.fill('[name=body]', 'Something broke');
    await page.click('button:has-text("建立")');
    await expect(page.locator('text=Bug report')).toBeVisible();
});
```

### Playwright 設定

```typescript
import { defineConfig } from '@playwright/test';
export default defineConfig({
    testDir: './specs',
    timeout: 60000,
    use: {
        baseURL: 'http://localhost:8080',
        headless: true,
        screenshot: 'only-on-failure',
    },
    webServer: {
        command: 'cargo run',
        port: 8080,
        timeout: 30000,
        reuseExistingServer: true,
    },
});
```

### 執行方式

```bash
cd frontend && npm install && npx playwright install
cd frontend && npx playwright test                      # 全部
cd frontend && npx playwright test --headed --trace on  # 有 UI
cd frontend && npx playwright show-report               # 報告
```

## 層級 6：Fuzzing 與屬性測試

### Rust Fuzzing

使用 `cargo-fuzz` 對輸入解析路徑進行模糊測試：

```rust
// fuzz/fuzz_targets/api_input.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(body) = std::str::from_utf8(data) {
        let _ = handlers::auth::register(/* mock state */, Json(body));
        let _ = handlers::files::write_file(/* mock state */, body);
    }
});
```

### 屬性測試（proptest）

對加密和路徑操作進行屬性測試：

```rust
proptest! {
    #[test]
    fn encrypt_decrypt_roundtrip(plaintext: String) {
        let (ct, nonce) = encrypt_secret(&plaintext).unwrap();
        let result = decrypt_secret(&ct, &nonce).unwrap();
        assert_eq!(plaintext, result);
    }

    #[test]
    fn safe_path_rejects_dotdot(path in ".*\\.\\..*") {
        assert!(safe_path(&path).is_err());
    }
}
```

## 測試自動化與 CI（GitHub Actions）

```yaml
name: Test
on: [push, pull_request]
jobs:
  unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test --lib

  integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: actions/setup-python@v5
      - run: cargo build && ./test/run_all.sh

  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - name: Install hurl
        run: |
          curl -LO https://github.com/Orange-OpenSource/hurl/releases/download/4.0.0/hurl_4.0.0_amd64.deb
          sudo dpkg -i hurl_4.0.0_amd64.deb
      - run: cargo build && cargo run &
      - run: ./tests/run_api_tests.sh

  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: actions/setup-node@v4
      - run: cargo build && cargo run &
      - run: cd frontend && npm install && npx playwright install chromium
      - run: cd frontend && npx playwright test

  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ./test_docker.sh
```

## 優先級與路線圖

| 階段 | 內容 | 工作量 | 依賴 |
|------|------|--------|------|
| **P0** | 單元測試：auth/config/errors/db 四個模組 | 2-3 天 | 無 |
| **P0** | 整合測試：拆 14 子腳本，補 File Manager/Settings/Issues/PRs | 2-3 天 | 無 |
| **P1** | REST API 測試：hurl 12 檔案 30+ 端點 | 1-2 天 | 安裝 hurl |
| **P1** | CLI 測試：5 場景 | 0.5 天 | 無 |
| **P2** | E2E 測試：Playwright 6 場景 | 3-4 天 | Node.js + Playwright |
| **P2** | 單元測試補完：handlers/ 下全部模組 | 3-5 天 | mock git repo |
| **P3** | Fuzzing + 屬性測試 | 2-3 天 | cargo-fuzz, proptest |
| **P3** | CI 自動化 + GitHub Actions | 1 天 | GitHub Actions |

## 測試涵蓋率矩陣

```
功能模組          │ Unit │ API  │ Integ │ E2E │ CLI │ Fuzz
─────────────────┼──────┼──────┼───────┼─────┼─────┼─────
Auth (register)   │  ✅  │  ✅  │   ✅  │  ✅ │     │
Auth (login/JWT)  │  ✅  │  ✅  │   ✅  │  ✅ │     │  ✅
Auth (SSH info)   │      │  ✅  │   ✅  │     │     │
Repo CRUD         │  ✅  │  ✅  │   ✅  │  ✅ │     │
Repo Search       │      │  ✅  │   ✅  │     │     │
Repo Fork         │      │  ✅  │   ✅  │     │     │
Content Tree      │  ✅  │  ✅  │   ✅  │     │     │
Content Blob      │      │  ✅  │   ✅  │     │     │
Content Readme    │      │  ✅  │   ✅  │     │     │
Content Commits   │      │  ✅  │   ✅  │     │     │
File Manager      │  ✅  │  ✅  │   ✅  │  ✅ │     │  ✅
Pages             │      │  ✅  │   ✅  │  ✅ │     │
Apps Deploy       │  ✅  │  ✅  │   ✅  │     │     │
Apps Deploy Log   │      │  ✅  │   ✅  │     │     │
Orgs CRUD         │  ✅  │  ✅  │   ✅  │  ✅ │     │
Orgs Members      │      │  ✅  │   ✅  │     │     │
Issues CRUD       │  ✅  │  ✅  │   ✅  │  ✅ │     │
Labels            │      │  ✅  │   ✅  │     │     │
Comments          │      │  ✅  │   ✅  │     │     │
PR CRUD           │  ✅  │  ✅  │   ✅  │     │     │
PR Merge (3-way)  │  ✅  │  ✅  │   ✅  │     │     │
PR Diff           │      │  ✅  │   ✅  │     │     │
Access Tokens     │  ✅  │  ✅  │   ✅  │     │     │
Collaborators     │      │  ✅  │   ✅  │     │     │
Secrets (AES)     │  ✅  │  ✅  │   ✅  │     │     │
Branch Protection │      │  ✅  │   ✅  │     │     │
Stars             │      │  ✅  │   ✅  │     │     │
Watches           │      │  ✅  │   ✅  │     │     │
SSH Keys          │      │  ✅  │   ✅  │     │     │
Git Push/Clone    │      │      │   ✅  │  ✅ │     │
Git HTTP Backend  │      │      │   ✅  │     │     │
Config Loading    │  ✅  │      │      │     │  ✅ │
Docker Containers │  ✅  │      │   ✅  │     │     │
CLI args          │      │      │      │     │  ✅ │
SPA Routing       │      │      │      │  ✅ │     │
Navigation Layout │      │      │      │  ✅ │     │
錯誤路徑         │  ✅  │  ✅  │   ✅  │     │  ✅ │  ✅
```
