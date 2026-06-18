# Argon2（密碼雜湊演算法）

## 概述

Argon2 是 2015 年 Password Hashing Competition（PHC）的冠軍得主，被設計為一種**記憶體硬性**（Memory-Hard）的密碼雜湊函數，旨在抵抗 GPU、ASIC、FPGA 等硬體的暴力破解攻擊。Gitpage 使用 `argon2` crate 對使用者密碼進行安全雜湊儲存。

## 設計動機

傳統的密碼儲存方式（純文字、MD5、SHA-1、SHA-256）存在嚴重安全問題：

- **彩虹表攻擊**：預先計算的雜湊值對照表可快速反查原文
- **GPU 平行加速**：MD5/SHA 系列的設計目標是快速，GPU 可每秒計算數十億次
- **ASIC 專用硬體**：專用晶片可進一步加速

為了解決這些問題，現代密碼雜湊演算法加入了兩種「硬度」：

### CPU 硬度（CPU-Hard）

透過增加迭代次數提高計算成本：

```
hash = H(H(H(...H(password + salt)...)))  // 數千至數百萬次迭代
```

典型代表：PBKDF2、bcrypt

### 記憶體硬度（Memory-Hard）

強制使用大量記憶體，使 GPU/ASIC 的平行優勢消失（每個計算單元需要獨立記憶體）：

```
memory = M[0..S]  // 分配 S 大小的記憶體
for i in 0..T:
    memory[i] = H(memory[i-1], memory[random_index])
hash = memory[last]
```

Argon2 同時具備 CPU 硬度和記憶體硬度。

## Argon2 的演算法細節

Argon2 有三個變體：

| 變體 | 特性 | 適用場景 |
|------|------|---------|
| **Argon2d** | 資料依賴的記憶體存取 | 加密貨幣挖礦（抗 GPU） |
| **Argon2i** | 資料獨立的記憶體存取 | 密碼雜湊（抗 side-channel） |
| **Argon2id** | 混合模式（前段 Argon2i + 後段 Argon2d） | **密碼雜湊推薦** |

Gitpage 使用 Argon2id，因其同時具備 Argon2i 的 side-channel 抵抗力和 Argon2d 的 GPU 抵抗力的優點。

### 核心參數

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::params::Params;

let params = Params::new(
    Params::DEFAULT_M_COST,      // 記憶體成本（64 MB = 65536 KiB）
    Params::DEFAULT_T_COST,      // 時間成本（3 次迭代）
    Params::DEFAULT_P_COST,      // 平行度（4 個執行緒）
    None,                        // 輸出長度（預設 32 bytes）
)?;

let argon2 = Argon2::new(
    argon2::Algorithm::Argon2id,
    argon2::Version::V0x13,      // v1.3（最新版本）
    params,
);
```

#### 參數意義

- **M Cost（記憶體成本）**：以 KiB 為單位。預設值 19456（19 MiB），最高可達 2^32-1 KiB
  - 每增加一倍，GPU 暴力破解成本增加一倍
  - Gitpage 使用預設值

- **T Cost（時間成本）**：迭代次數。預設值 2，建議至少 3
  - 影響 CPU 計算時間
  - 增加 1 次迭代約略增加 50% 計算時間

- **P Cost（平行度）**：並行執行緒數。預設值 1，建議最多 4
  - 對抗 GPU 的關鍵參數之一

### 密碼雜湊流程

#### 註冊（建立雜湊）

```rust
// src/handlers/auth.rs
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<Value>, AppError> {
    // 1. 驗證輸入
    if body.password.len() < 6 {
        return Err(AppError::BadRequest("密碼至少需要6個字元".into()));
    }

    // 2. 產生 salt（自動由 argon2 crate 產生 crypto-safe 隨機 salt）
    let salt = SaltString::generate(&mut OsRng);

    // 3. 計算雜湊（Argon2id + salt）
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 4. 儲存 PHC 格式字串到資料庫
    let hash_str = hash.to_string();
    // hash_str 格式: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
    state.db.create_user(&body.username, &body.email, &hash_str)?;

    Ok(Json(json!({ "success": true })))
}
```

#### 登入（驗證密碼）

```rust
// src/handlers/auth.rs
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<Value>, AppError> {
    // 1. 查找使用者
    let user = state.db.get_user_by_username(&body.username)?
        .ok_or(AppError::Unauthorized("使用者名稱或密碼錯誤".into()))?;

    // 2. 從資料庫讀取儲存的雜湊（PHC 格式）
    let stored_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 3. 驗證密碼
    let argon2 = Argon2::default();
    let valid = argon2.verify_password(
        body.password.as_bytes(),
        &stored_hash,
    ).is_ok();

    if !valid {
        return Err(AppError::Unauthorized("使用者名稱或密碼錯誤".into()));
    }

    // 4. 產生 JWT
    let token = create_token(user.id, &user.username, state.jwt_expires_hours)?;

    Ok(Json(json!({
        "token": token,
        "user": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
        }
    })))
}
```

## PHC 字串格式

Argon2 雜湊以 PHC（Password Hashing Competition）字串格式儲存：

```
$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$YEt4O0K7v0F0P5g0q2b3w4r5t6y7u8i9o0p1a2s3d4f5
│         │    │                     │          │
演算法    版本  參數                  salt       hash（base64 編碼）
```

此格式包含所有需要的資訊，驗證時無需額外儲存 salt 或參數。

```rust
// 各種欄位說明
$argon2id           // 演算法（Argon2id）
$v=19              // 版本（0x13 = 19）
$m=19456,t=2,p=1   // 參數：m=記憶體(KiB), t=迭代次數, p=平行度
$<base64-salt>     // 隨機 salt（16 bytes）
$<base64-hash>     // 最終雜湊值（32 bytes）
```

## Argon2 vs 其他演算法

| 演算法 | 記憶體硬度 | 預設成本 | GPU 抵抗 | Side-Channel 抵抗 | 密碼長度限制 |
|--------|-----------|---------|---------|------------------|------------|
| **Argon2id** | ✅ 高 | 64MB+3iter+4thread | ✅ 強 | ✅ 強 | 無限制 |
| **bcrypt** | ❌ 低 | 4KB | ❌ 弱 | ❌ | 72 bytes |
| **PBKDF2** | ❌ 無 | 600K iterations | ❌ 極弱 | ✅ | 無限制 |
| **scrypt** | ✅ 中 | 16MB+1iter | ✅ 中 | ❌ | 無限制 |
| **SHA-256 (x1)** | ❌ 無 | 1 iteration | ❌ 極弱 | ✅ | 無限制 |

## 安全性最佳實踐

### 1. 永遠不要限制密碼長度（或至少 128 字元）

Argon2 無密碼長度上限，應允許長密碼（passphrase）。

### 2. 使用 salt

Argon2 自動產生 crypto-safe 的隨機 salt，確保相同密碼產生不同雜湊。

### 3. 調整參數

參數應隨著硬體效能提升而增加。經驗法則：讓驗證時間在 50-100ms 之間。

```rust
// 高安全性設定（適合敏感系統）
let params = Params::new(
    65536,   // 64 MiB 記憶體
    5,       // 5 次迭代
    4,       // 4 個執行緒
    Some(32),// 32 bytes 輸出
)?;
```

### 4. 時序攻擊防護

Argon2 的驗證時間取決於參數而非密碼長度，因此不易從回應時間推測資訊。

### 5. 資料庫洩漏時的影響

即使資料庫被駭，攻擊者仍需要：
- 對每個密碼猜測執行完整的 Argon2 計算（約 50-100ms）
- 嘗試 10 億個常見密碼需：10^9 × 0.1s ÷ 3600 ÷ 24 ≈ 1157 天（單執行緒）
- 即使使用 1000 個 GPU 平行計算，也需要 ~28 小時

## Rust 實作細節

### Cargo.toml

```toml
[dependencies]
argon2 = "0.5"
```

### 密碼變更

```rust
// src/handlers/auth.rs
pub async fn change_password(
    State(state): State<AppState>,
    axum::Extension(user_id): axum::Extension<i64>,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, AppError> {
    let user = state.db.get_user(user_id)?
        .ok_or(AppError::NotFound("使用者不存在".into()))?;

    // 先驗證舊密碼
    let stored_hash = PasswordHash::new(&user.password_hash)?;
    let argon2 = Argon2::default();
    if argon2.verify_password(body.current_password.as_bytes(), &stored_hash).is_err() {
        return Err(AppError::BadRequest("目前密碼錯誤".into()));
    }

    // 計算新密碼的雜湊
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = argon2.hash_password(body.new_password.as_bytes(), &salt)?;

    state.db.update_password(user_id, &new_hash.to_string())?;
    Ok(Json(json!({ "success": true })))
}
```

## 參考資料

- [Argon2 RFC 9106](https://datatracker.ietf.org/doc/rfc9106/)
- [Password Hashing Competition](https://password-hashing.net/)
- [Argon2 crate](https://crates.io/crates/argon2)
- [OWASP - Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- `src/handlers/auth.rs` — 註冊/登入/密碼變更實作
- `src/db/mod.rs` — 使用者 CRUD（儲存 password_hash）
