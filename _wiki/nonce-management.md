# AES-256-GCM Nonce 管理策略

## 概述

Nonce（Number used ONCE）是 AES-256-GCM 認證加密中的一個關鍵參數，決定了加密的安全性。一個 96-bit（12 bytes）的 nonce 與密鑰配對使用，確保每一次加密的金鑰串流都是獨一無二的。Gitpage 在加密使用者 Secrets 時，採用隨機 nonce 生成策略，並將 nonce 與密文一起儲存在資料庫中。

## Nonce 的定義與功能

### GCM 模式中的 Nonce

GCM（Galois/Counter Mode）使用 CTR 模式進行加密，並加上 GHASH 進行認證。Nonce 在 GCM 中的角色如下：

```
Nonce (96-bit / 12 bytes)
     │
     └── 與 Counter 拼接
              │
              └── J0 = Nonce || Counter(0)   (初始計數器)
              │
              ├── J1 = Nonce || Counter(1)   → AES_K(J1) → keystream block 1
              ├── J2 = Nonce || Counter(2)   → AES_K(J2) → keystream block 2
              │   ...
              └── Jn = Nonce || Counter(n)   → AES_K(Jn) → keystream block n

Plaintext  ⊕  keystream = Ciphertext
```

### Nonce 決定金鑰串流

CTR 模式的核心是：**Nonce 不同 → 初始計數器不同 → 所有 keystream block 不同**。同一密鑰下，只要 nonce 不同，加密同一段明文就會產生完全不同的密文。

```
AES-KEY(key, nonce1) → keystream_A
AES-KEY(key, nonce2) → keystream_B  (與 keystream_A 完全無關)
```

## Nonce 重用的災難性後果

### 第一層攻擊：金鑰串流重複

如果同一個密鑰下使用了相同的 nonce 加密兩個不同的明文：

```
C1 = P1 ⊕ keystream
C2 = P2 ⊕ keystream  (same nonce → same keystream)

C1 ⊕ C2 = (P1 ⊕ keystream) ⊕ (P2 ⊕ keystream) = P1 ⊕ P2
```

攻擊者無需知道密鑰，只需對兩個密文做 XOR，就能得到兩個明文的 XOR。透過語言統計分析，攻擊者可以輕鬆恢復兩個明文的內容。

### 第二層攻擊：認證金鑰 H 的恢復

GCM 的 GHASH 使用認證金鑰 `H = AES_K(0^128)`。如果 nonce 重用，攻擊者可以：

1. 收集使用相同 nonce 加密的 `(C, Tag)` 對
2. 透過 GHASH 的線性特性建立方程式
3. 求解認證金鑰 H
4. 偽造任意資料的認證標籤

一旦 H 被恢復，攻擊者可以構造任意有效的 `(C, Tag)` 對，完全繞過完整性保護。

### 實際案例

Nonce 重用攻擊不是理論上的。2016 年，`aes-gcm` 函式庫的 Java 實作（`AES/GCM/NoPadding`）被發現當密鑰長度為 256-bit 時，nonce 生成有 bug，導致 AWS 的多個服務使用了重複的 nonce。這也展示了為什麼認證加密的安全稽核如此重要。

## Gitpage 的隨機 Nonce 生成

### 實作方式

Gitpage 使用 `aes-gcm` crate 提供的 `generate_nonce` 方法：

```rust
use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit, OsRng}};

pub fn encrypt_secret(plaintext: &str) -> Result<(String, String), AppError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    // 生成 96-bit 隨機 nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // 加密
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 回傳 base64 編碼的密文和 nonce
    Ok((
        base64::encode(&ciphertext),
        base64::encode(&nonce),
    ))
}
```

### OsRng 隨機數生成器

`OsRng` 使用作業系統提供的安全隨機數源：
- **Linux**：`/dev/urandom`（或 `getrandom()` 系統呼叫）
- **macOS**：`SecRandomCopyBytes()` / `getentropy()`
- **Windows**：`CryptGenRandom()` / `BCryptGenRandom()`

這些都是經過安全稽核的 CSPRNG（Cryptographically Secure Pseudo-Random Number Generator）。

## 碰撞機率分析

### 生日悖論

隨機 nonce 的碰撞問題可用生日悖論分析。對於 96-bit 的 nonce：

- 單次碰撞機率：`2^(-96)`（約 10^(-29)）
- 在 N 次加密後出現碰撞的機率近似於：
  ```
  P ≈ N^2 / (2 * 2^96)
  ```

### 安全的加密次數上限

如果我們要求碰撞機率低於 2^(-60)（一個極安全的門檻）：

```
N^2 / (2 * 2^96) ≤ 2^(-60)
N^2 ≤ 2^37
N ≤ 2^18.5 ≈ 370,000
```

也就是說，在同一個密鑰下，可以安全地加密約 37 萬次 Secrets 而不必擔心 nonce 碰撞。對於 Gitpage 的使用場景（每個使用者可能儲存幾十個 Secrets，修改次數有限），這遠遠超過了實際需求。

### 對比不同 nonce 長度

| Nonce 長度 | 隨機 nonce 的安全容量 (P < 2^(-60)) | 特性 |
|-----------|-------------------------------------|------|
| 64-bit | N ≤ 2,048 次 | 不建議用隨機 nonce |
| 96-bit | N ≤ 370,000 次 | GCM 建議值（標準） |
| 128-bit | N ≤ 2.5×10^10 次 | 需要 GHASH 處理 |

## Nonce 儲存策略

### 前置串接（Gitpage 的實作）

Gitpage 將 nonce 與密文分開儲存在資料庫中：

```sql
CREATE TABLE repo_secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,  -- base64 編碼的密文
    nonce TEXT NOT NULL,            -- base64 編碼的 nonce
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, name)
);
```

備選方案是將 nonce 前置在密文之前：

```
儲存格式：nonce (12 bytes) || ciphertext (N bytes) || tag (16 bytes)
```

這種方案的優點是只需儲存一個 base64 字串而非兩個。Gitpage 選擇分開儲存是為了資料庫查詢的可讀性和除錯方便。

### 解密時的 nonce 使用

```rust
pub fn decrypt_secret(ciphertext_b64: &str, nonce_b64: &str) -> Result<String, AppError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let ciphertext = base64::decode(ciphertext_b64)?;
    let nonce_bytes = base64::decode(nonce_b64)?;

    // 從 12 bytes 建立 Nonce 物件
    let nonce = Nonce::from_slice(&nonce_bytes);

    // decrypt 會自動驗證 GHASH tag
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Internal(e.to_string()))
}
```

## Nonce 長度考量

### GCM 標準 nonce：12 bytes

NIST SP 800-38D 建議 nonce 長度為 96-bit（12 bytes），理由是：

1. **可直接用作初始計數器 J0**：`J0 = Nonce || 0^32 || 1`，不需要額外的 GHASH 處理
2. **效能最佳**：避免 GHASH 從 nonce 轉換為計數器的開銷
3. **廣泛支援**：所有 GCM 實作都必須支援

### 非標準長度 nonce

如果 nonce 不是 12 bytes，GCM 使用 GHASH 將其轉換為 96-bit 計數器：

```
If len(Nonce) != 12:
    J0 = GHASH(H, {}, Nonce || 0^(s+64) || len(Nonce)_64)
```

其中 s = (128 * ceil(len(Nonce)/128)) - len(Nonce)，H 是認證金鑰。

這會帶來：
1. 額外的 GHASH 計算（對小資料來說可能是顯著的效能損耗）
2. 潛在的 GHASH 碰撞風險

### Gitpage 的選擇

Gitpage 使用標準的 12 bytes（96-bit）nonce，由 `Aes256Gcm::generate_nonce(&mut OsRng)` 自動產生。`aes-gcm` crate 的 `generate_nonce` 回傳的正好是 12 bytes。

## 隨機 Nonce vs 計數器式 Nonce

### 隨機 Nonce（Gitpage 使用）

```rust
let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
// 每次加密產生不同的 96-bit 隨機數
```

優點：
- **無需狀態管理**：每個加密操作獨立，不依賴前次操作
- **平行安全**：多個執行緒可以同時加密而不需要同步
- **崩潰安全**：伺服器崩潰後重啟不會導致 nonce 重複

缺點：
- **碰撞風險**（雖然機率極低）
- **需要 CSPRNG**：如果隨機數品質不佳，碰撞機率增加

### 計數器式 Nonce（Deterministic）

```rust
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn get_nonce() -> [u8; 12] {
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut nonce = [0u8; 12];
    nonce[4..].copy_from_slice(&count.to_be_bytes());
    nonce
}
```

優點：
- **零碰撞**：只要計數器正確維護，永遠不會重複
- **無需隨機數源**

缺點：
- **狀態管理**：計數器需要持久化，否則重啟後會重複
- **平行寫入**：多執行緒環境需要同步存取
- **寫入失敗**：如果加密成功但儲存失敗，nonce 被消耗但未被使用

### 比較總結

| 特性 | 隨機 Nonce | 計數器 Nonce |
|------|-----------|-------------|
| 碰撞機率 | 2^(-96) 每次 | 零（正常操作） |
| 狀態持久化 | 不需要 | 需要 |
| 平行安全 | ✅ 天生支援 | ⚠️ 需原子操作 |
| 崩潰復原 | ✅ 安全 | ⚠️ 可能重複 |
| 實作複雜度 | 低（一行程式碼） | 中 |
| 依賴 | CSPRNG（由 OsRng 提供） | 持久化儲存 |

對於 Gitpage 的 Secrets 使用場景（加密次數極少），隨機 nonce 是正確且安全的選擇。

## 最佳實踐

### 金鑰輪換

Nonce 只在同一個密鑰下才有碰撞問題。如果定期輪換密鑰，即使 nonce 碰撞的機率進一步降低：

```rust
pub struct EncryptedSecret {
    key_version: u32,          // 密鑰版本
    ciphertext_b64: String,
    nonce_b64: String,
}
```

### Nonce 的完整性保護

Nonce 本身不加密，但它的完整性由 GCM 的認證標籤（tag）保護。如果 nonce 在儲存過程中被篡改，解密時 GHASH 驗證會失敗：

```rust
// Nonce 被篡改 → decrypt 回傳 Err
cipher.decrypt(wrong_nonce, ciphertext.as_ref())  // Err
```

### 避免 nonce 來源污染

永遠不要使用：
- `rand::thread_rng()`（非密碼學安全）
- `std::time` 為基礎的 nonce（可預測）
- 使用者輸入作為 nonce

只使用 `OsRng` 或經過認證的 CSPRNG。

## 實際案例：Gitpage 的 Nonce 重用測試

```rust
#[test]
fn test_nonce_uniqueness() {
    let key = [0u8; 32];
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));

    let mut seen = std::collections::HashSet::new();
    for _ in 0..10000 {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        assert!(seen.insert(nonce.to_vec()));
    }
    // 在 10000 次隨機生成中無碰撞
}
```

當然，真正需要的是數學保證而非測試。對於 96-bit 隨機 nonce，10000 次無碰撞的機率為：

```
P(no collision) = ∏_{i=0}^{9999} (1 - i/2^96)
                ≈ e^(-10^8 / 2^97)
                ≈ 1 - 10^8 / 2^97
                ≈ 1 - 6.3 × 10^(-22)
```

基本上就是 100%。

## 參考資料

- [NIST SP 800-38D - GCM](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf) — GCM 規格（nonce 處理在第 5.2.1 節）
- [RFC 5116 - AEAD](https://datatracker.ietf.org/doc/rfc5116/) — AEAD 介面定義
- [aes-gcm crate](https://crates.io/crates/aes-gcm) — Rust 的 AES-GCM 實作
- [Nonce 重用攻擊論文](https://www.usenix.org/legacy/events/sec08/tech/full_papers/ono/ono_html/) — 實際的 nonce 重用攻擊分析
- `src/handlers/settings.rs:240-270` — encryt/decrypt 實作（nonce 生成與恢復）
- `src/auth/mod.rs:70-82` — `ENCRYPTION_KEY` 初始化
- `_wiki/aes-256-gcm.md` — AES-256-GCM 完整理論背景
