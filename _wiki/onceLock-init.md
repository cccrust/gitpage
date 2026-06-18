# OnceLock（Rust 延遲全域初始化）

## 概述

`OnceLock` 是 Rust 標準庫中的同步原語（自 1.70 穩定），用於實作**一次性初始化**的全域變數。與 `lazy_static` 或 `once_cell` 類似，`OnceLock` 允許在全域範圍定義一個變數，在首次存取時初始化，之後保持不變。Gitpage 使用 `OnceLock` 管理 JWT 密鑰和加密密鑰這類需要在啟動時設定、但之後唯讀的全域資源。

## 問題背景

在 Gitpage 中，JWT 密鑰和 AES 加密密鑰有以下特性：

1. **全域存取**：在認證中間件、handler、工具函數等各處都需要使用
2. **啟動時決定**：值從 `config.toml` 或環境變數讀取，編譯時未知
3. **設定後唯讀**：伺服器執行期間不應變更
4. **多執行緒安全**：需要在多個 tokio 任務中安全共享

傳統上 Rust 開發者使用 `lazy_static!` 或 `once_cell::sync::Lazy` 解決此問題。現在標準庫的 `OnceLock` 提供了同樣的功能。

## OnceLock 用法

### 標準模式

```rust
use std::sync::OnceLock;

static JWT_SECRET: OnceLock<String> = OnceLock::new();
```

### 初始化（僅能執行一次）

```rust
pub fn init_jwt_secret(config_secret: &str) {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| config_secret.to_string());
    JWT_SECRET.set(secret).ok();  // .ok() 忽略重複設定的錯誤
}
```

### 讀取

```rust
pub fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get().expect("JWT 密鑰未初始化")
}
```

## Gitpage 的應用

### JWT 密鑰

```rust
// src/auth/mod.rs
static JWT_SECRET: OnceLock<String> = OnceLock::new();
static ENCRYPTION_KEY: OnceLock<[u8; 32]> = OnceLock::new();

pub fn init(config: &Config) {
    // 初始化 JWT 密鑰
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| config.jwt.secret.clone());
    JWT_SECRET.set(jwt_secret).expect("JWT 密鑰重複初始化");

    // 初始化加密密鑰（用於 Secrets 加密）
    let key_material = if !config.secrets.encryption_key.is_empty() {
        config.secrets.encryption_key.clone()
    } else {
        config.jwt.secret.clone()
    };
    let key = sha2::Sha256::digest(key_material.as_bytes());
    ENCRYPTION_KEY.set(key.into()).expect("加密密鑰重複初始化");
}
```

### Token 建立與驗證

```rust
pub fn create_token(user_id: i64, username: &str, expires_in_hours: u64) -> Result<String, AppError> {
    let secret = JWT_SECRET.get().ok_or_else(|| {
        AppError::Internal("JWT 密鑰未初始化".into())
    })?;

    let claims = Claims { ... };
    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let secret = JWT_SECRET.get().ok_or_else(|| {
        AppError::Internal("JWT 密鑰未初始化".into())
    })?;

    let data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
```

### Secrets 加密

```rust
pub fn encrypt_secret(plaintext: &str) -> Result<(String, String), AppError> {
    let key = ENCRYPTION_KEY.get().ok_or_else(|| {
        AppError::Internal("加密密鑰未初始化".into())
    })?;

    use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit, OsRng}};
    let cipher = Aes256Gcm::new(From::from(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())?;

    Ok((base64::encode(ciphertext), base64::encode(nonce)))
}
```

## OnceLock vs 其他方案

| 方案 | 標準庫 | 開銷 | 語法 | 額外功能 |
|------|--------|------|------|---------|
| **`std::sync::OnceLock`** | ✅ 是 | 低 | 靜態方法 | 無 |
| **`once_cell::sync::Lazy`** | ❌ 第三方 | 低 | Lazy! 巨集 | 自動初始化 |
| **`lazy_static!`** | ❌ 第三方 | 中 | 巨集 | 支援任何表達式 |
| **`std::sync::Once`** | ✅ 是 | 最低 | 回呼風格 | 僅初始化一次 |
| **`std::sync::LazyLock`** | ✅ Nightly | 低 | 靜態方法 | 自動初始化 |

### OnceLock vs LazyLock

- `OnceLock`：需要手動呼叫 `.set(value)` 初始化
- `LazyLock`（Rust 1.80 穩定）：在首次 `.get()` 時自動初始化

```rust
// LazyLock（自動初始化）
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::load("config.toml").unwrap()
});

// OnceLock（手動初始化）
static JWT_SECRET: OnceLock<String> = OnceLock::new();
fn init() { JWT_SECRET.set(value).ok(); }
fn use() { JWT_SECRET.get().unwrap() }
```

Gitpage 使用 `OnceLock` 而非 `LazyLock`，因為密鑰的設定時機在應用生命週期中是明確的（啟動時），且希望在所有程式碼執行前完成初始化，而非在 handler 首次執行時才初始化。

## 初始化順序保證

```rust
// main.rs
fn main() {
    // 1. 載入設定
    let config = load_config();

    // 2. 初始化全域密鑰（OnceLock.set()）
    auth::init(&config);

    // 3. 初始化資料庫
    let db = Database::new(&config.database.path);

    // 4. 建立應用
    let app = create_app(state);

    // 5. 啟動伺服器
    // → 此時所有 OnceLock 都已初始化
    // → handler 中呼叫 get() 不會 panic
}
```

## 執行緒安全

`OnceLock` 內部使用 `std::sync::Once` 確保：

1. **最多執行一次**：即使多個執行緒同時呼叫 `set()`，只有第一個會成功
2. **記憶體屏障**：初始化後的讀取保證看到完整的初始化狀態
3. **不阻塞**：初始化完成後，`get()` 是原子載入，無鎖競爭

```rust
fn thread_safe_example() {
    std::thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                // 多個執行緒同時呼叫 get()
                // 如果尚未初始化，可能都看到 None
                // 但不會造成 data race
                if let Some(val) = JWT_SECRET.get() {
                    println!("{}", val);
                }
            });
        }
    });
}
```

## 限制與注意事項

1. **不可變**：初始化後無法修改或重設值
2. **Panic 風險**：未初始化就 `get().unwrap()` 會 panic
3. **Mutex 替代**：如需執行期間動態變更，應使用 `RwLock` 或 `Mutex`
4. **測試**：測試間共享的 `OnceLock` 可能造成測試狀態汙染

```rust
// 測試時的處理方式
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_keys() {
        // 每次測試前確保初始化
        init_jwt_secret("test-secret-for-testing");
        let token = create_token(1, "test", 1).unwrap();
        assert!(verify_token(&token).is_ok());
    }
}
```

## 參考資料

- [OnceLock 文件](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)
- [LazyLock RFC](https://rust-lang.github.io/rfcs/2788-standard-lazy-types.html)
- `src/auth/mod.rs` — Gitpage 的 JWT 和加密密鑰初始化
