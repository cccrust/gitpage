# utils/errors.rs — AppError Enum and Response

## Theoretical Background

### AppError Enum Design

The `AppError` enum models all application-level errors with five variants, each carrying a human-readable message string:

| Variant | HTTP Status | Semantics |
|---|---|---|
| `NotFound` | 404 | Resource does not exist |
| `Unauthorized` | 401 | Authentication or permission failure |
| `BadRequest` | 400 | Invalid input or malformed request |
| `Internal` | 500 | Unexpected server-side failure |
| `Conflict` | 409 | Duplicate or state conflict |

Messages are written in Chinese, matching the frontend's UI language. Each variant's message is context-specific, describing exactly what went wrong (e.g., `"找不到分支 'main'"`).

### IntoResponse for Axum

The `impl IntoResponse for AppError` is what makes `AppError` usable as a return type from Axum handlers. It converts each variant to the corresponding HTTP status code and produces a JSON body:

```json
{"error": "<message>"}
```

The response format is always `(StatusCode, Json<Value>)`, ensuring consistent error shapes across the entire API. Axum's `IntoResponse` trait allows handlers to use `Result<T, AppError>` directly, without manual error mapping.

### From Trait Implementations for Automatic Conversion

Three `From` implementations enable the `?` operator to automatically convert lower-level errors into `AppError`:

- **`From<rusqlite::Error>`** — wraps database errors as `AppError::Internal("資料庫錯誤: ...")`
- **`From<git2::Error>`** — wraps libgit2 errors as `AppError::Internal("Git 錯誤: ...")`
- **`From<std::io::Error>`** — wraps I/O errors as `AppError::Internal("IO 錯誤: ...")`

This means database calls, git operations, and file I/O in handlers can use `?` directly, and any error that reaches the Axum error handler is guaranteed to be an `AppError` with a descriptive Chinese message. The pattern means `AppError` serves as a unified error boundary: all errors from lower layers are caught, wrapped, and converted into a consistent API response format.

### Display Implementation

The `Display` trait implementation provides formatted Chinese error strings, prefixed by the error category. This is used for logging and debug output, not for API responses (which use the JSON format from `IntoResponse`).

## References

- See `_wiki: apperror-pattern.md` for the unified error handling architecture
