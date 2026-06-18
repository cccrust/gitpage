# AES-256-GCM（認證加密）

## 概述

AES-256-GCM（Advanced Encryption Standard 256-bit, Galois/Counter Mode）是一種**認證加密**（Authenticated Encryption with Associated Data, AEAD）演算法，同時提供資料的機密性（Confidentiality）和完整性（Integrity）。Gitpage 使用 AES-256-GCM 加密使用者儲存在資料庫中的 Secrets（CI/CD 環境變數），確保即使資料庫被洩露，Secrets 內容也不會被破解。

## 演算法背景

### AES（高階加密標準）

AES 是由 Joan Daemen 和 Vincent Rijmen 設計的區塊加密演算法，2001 年被 NIST 採納為聯邦標準（FIPS 197）。AES 使用 128-bit 的區塊大小和 128/192/256-bit 的密鑰長度。

AES 的核心是**替代-排列網路**（Substitution-Permutation Network）：

```
plaintext (128-bit)
    │
    ├─ AddRoundKey (round 0)
    │
    ├─ Round 1..N:         ──► SubBytes (S-box 替代)
    │                          ├─ ShiftRows (列位移)
    │                          ├─ MixColumns (列混合)
    │                          └─ AddRoundKey (加金鑰)
    │
    ├─ Final Round:        ──► SubBytes
    │                          ├─ ShiftRows
    │                          └─ AddRoundKey
    │
    ▼
ciphertext (128-bit)
```

### GCM（Galois/Counter Mode）

GCM 是一種**區塊加密的操作模式**，將區塊加密轉換為支援認證加密的串流加密：

```
                      Nonce (12 bytes)
                          │
                     Counter(0) ──► AES ──► Counter(1) ──► AES ──► ...
                          │                    │
                     GHASH(H, A, C)           Plaintext ⊕ Counter(1) → Ciphertext
                          │
                        Tag (認證標籤)
```

GCM 的關鍵特性：

1. **CTR 模式基礎**：使用 AES 加密計數器產生金鑰串流（keystream），與明文 XOR 產生密文
2. **內建認證**：透過 GHASH（Galois field multiplication）計算認證標籤，提供完整性驗證
3. **平行化**：CTR 模式的加密可平行計算
4. **Associated Data**：可附加不需加密但需認證的資料（如資料庫 ID）

GCM 的輸出為：

```
Ciphertext (與明文相同長度) + Tag (16 bytes)
```

## Gitpage 的 Secrets 加密實作

實作於 `src/handlers/settings.rs` 和 `src/auth/mod.rs`。

### 密鑰管理

加密密鑰使用 SHA-256 從使用者提供的密鑰字串推導，並儲存在全域的 `OnceLock` 中：

```rust
// src/auth/mod.rs
static ENCRYPTION_KEY: OnceLock<[u8; 32]> = OnceLock::new();

pub fn init_encryption_key(config: &Config) {
    // 優先使用 [secrets] encryption_key，否則 fallback 到 JWT 密鑰
    let key_str = if !config.secrets.encryption_key.is_empty() {
        &config.secrets.encryption_key
    } else {
        &config.jwt.secret
    };
    // SHA-256 推導為 32 bytes (256-bit) 密鑰
    let key = sha2::Sha256::digest(key_str.as_bytes());
    ENCRYPTION_KEY.set(key.into()).ok();
}
```

### 加密

```rust
use aes_gcm::{
    Aes256Gcm,
    Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use aes_gcm::aead::generic_array::GenericArray;

pub fn encrypt_secret(plaintext: &str) -> Result<(String, String), AppError> {
    let key = ENCRYPTION_KEY.get().ok_or_else(|| {
        AppError::Internal("加密密鑰未初始化".into())
    })?;

    // 1. 建立 cipher instance
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    // 2. 產生隨機 nonce（96-bit / 12 bytes — GCM 建議值）
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // 3. 加密（附帶資料為空）
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 4. 回傳 base64 編碼的密文和 nonce（分開儲存）
    Ok((
        base64::encode(&ciphertext),  // 密文
        base64::encode(&nonce),       // nonce
    ))
}
```

### 解密

```rust
pub fn decrypt_secret(ciphertext_b64: &str, nonce_b64: &str) -> Result<String, AppError> {
    let key = ENCRYPTION_KEY.get().ok_or_else(|| {
        AppError::Internal("加密密鑰未初始化".into())
    })?;

    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    // 1. 解碼 base64
    let ciphertext = base64::decode(ciphertext_b64)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let nonce_bytes = base64::decode(nonce_b64)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 2. 建立 Nonce 物件（GCM 使用 12 bytes nonce）
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 3. 解密（自動驗證完整性）
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Internal(e.to_string()))
}
```

### 資料庫儲存

在資料庫中，Secrets 以三欄位儲存：

```sql
CREATE TABLE repo_secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL,
    name TEXT NOT NULL,                  -- Secret 名稱（明文）
    encrypted_value TEXT NOT NULL,       -- base64 編碼的密文
    nonce TEXT NOT NULL,                -- base64 編碼的 nonce
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (repo_id) REFERENCES repositories(id) ON DELETE CASCADE,
    UNIQUE(repo_id, name)
);
```

### 完整 CRUD

```rust
// 建立 secret
pub async fn create_secret(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
    Json(body): Json<CreateSecretRequest>,
) -> Result<Json<Value>, AppError> {
    // 加密後儲存
    let (encrypted_value, nonce) = encrypt_secret(&body.value)?;
    state.db.create_repo_secret(repo_id, &body.name, &encrypted_value, &nonce)?;
    Ok(Json(json!({ "secret": { "name": body.name } })))
}

// 讀取 secret（解密後回傳）
pub async fn list_secrets(
    State(state): State<AppState>,
    Path(repo_id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let secrets = state.db.list_repo_secrets(repo_id)?;
    let decrypted: Vec<_> = secrets.into_iter().map(|s| {
        let value = decrypt_secret(&s.encrypted_value, &s.nonce)?;
        Ok(SecretResponse { id: s.id, name: s.name, value })
    }).collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(json!({ "secrets": decrypted })))
}
```

## GCM 的安全考量

### Nonce 重用

GCM 的致命弱點是 **nonce 絕對不可重用**。同一個密鑰下，重用 nonce 將導致：

1. 金鑰串流重複，可透過 `c1 ⊕ c2 = p1 ⊕ p2` 推測明文
2. 認證金鑰 H 被推估，所有認證標籤可偽造

Gitpage 使用 `OsRng`（作業系統安全亂數）產生 96-bit nonce，碰撞機率為 `2^(-96)`，可忽略不計。

### 認證標籤

GCM 在解密時自動驗證完整性：

```rust
// 如果密文或 nonce 被篡改，decrypt 會回傳 Err
let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
    .map_err(|_| AppError::BadRequest("Secret 資料已損毀".into()))?;
```

這確保了：
- 密文在傳輸或儲存過程中未被修改
- 使用的是正確的密鑰
- Nonce 與密文匹配

### 密鑰輪換

目前 Gitpage 未實作密鑰輪換機制。如果密鑰被洩露，所有加密的 Secrets 都需要重新加密。建議的改善方向：

```rust
// 密鑰版本標記（future work）
pub struct EncryptedSecret {
    key_version: u32,          // 密鑰版本
    ciphertext_b64: String,
    nonce_b64: String,
}
```

## 認證加密 vs 純加密

| 特性 | AES-256-GCM（認證加密） | AES-256-CBC（純加密） |
|------|------------------------|---------------------|
| 機密性 | ✅ | ✅ |
| 完整性 | ✅（GHASH 認證標籤） | ❌ 需額外 HMAC |
| 平行化 | ✅（CTR 模式） | ❌（CBC 序列） |
| 實現複雜度 | 低（單一演算法） | 高（加密 + HMAC） |
| 效能 | 高（硬體加速指令 AES-NI + PCLMULQDQ） | 中等 |

在現代 CPU 上，AES-256-GCM 的硬體加速可達到數十 Gbps 的吞吐量。

## 測試向量

驗證實作正確性的標準測試：

```rust
#[test]
fn test_aes_256_gcm() {
    let key = hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap();
    let plaintext = b"Hello, Gitpage Secrets!";
    let nonce = hex::decode("000102030405060708090a0b").unwrap();

    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let ciphertext = cipher.encrypt(
        Nonce::from_slice(&nonce),
        plaintext.as_ref(),
    ).unwrap();

    let decrypted = cipher.decrypt(
        Nonce::from_slice(&nonce),
        ciphertext.as_ref(),
    ).unwrap();

    assert_eq!(plaintext.to_vec(), decrypted);
}
```

## 參考資料

- [NIST FIPS 197 - AES](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.197.pdf)
- [NIST SP 800-38D - GCM](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- [aes-gcm crate](https://crates.io/crates/aes-gcm)
- [RFC 5116 - AEAD](https://datatracker.ietf.org/doc/rfc5116/)
- `src/auth/mod.rs` — `ENCRYPTION_KEY` 初始化
- `src/handlers/settings.rs` — Secrets CRUD（加密/解密）
