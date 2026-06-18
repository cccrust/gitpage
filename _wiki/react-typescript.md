# React 19 + TypeScript（前端架構）

## 概述

Gitpage 的前端使用 React 19 和 TypeScript 6 建構，搭配 Vite 8 作為建置工具。整體架構遵循**極簡主義**：無狀態管理函式庫、無 CSS-in-JS、無前端路由套件以外的依賴。這種設計選擇使得程式碼易於理解和維護，同時保持足夠的表達力來實作完整的 Git 平台 UI。

## React 19 基礎

### React 19 的關鍵特性

雖然 Gitpage 沒有使用 React 19 的所有新功能，但專案建基於 React 19 的穩定基底：

1. **並發模式（Concurrent Features）**：React 19 將並發渲染作為預設行為，允許 React 在渲染過程中中斷和恢復工作。這對 Gitpage 這種需要處理大量檔案列表頁面的大型渲染有潛在好處。
2. **自動批次更新（Automatic Batching）**：多個 `setState` 呼叫會自動批次處理，減少不必要的重新渲染。
3. **use() Hook**：直接在 render 中讀取 Promise 或 Context。
4. **Server Components**：雖然 Gitpage 未使用（這是 SPA，非 Next.js），但 React 19 的 Server Components 為未來 SSR 遷移保留了可能性。

Gitpage 使用的主要 React API：

```tsx
// useState — 元件內區域狀態
const [repo, setRepo] = useState<Repo | null>(null)
const [loading, setLoading] = useState(true)
const [err, setErr] = useState('')

// useEffect — 副作用（API 請求）
useEffect(() => {
    if (!id) return
    fetchData(id).then(...).catch(...)
}, [id])

// useRef — DOM 引用
const ref = useRef<HTMLDivElement>(null)
```

## TypeScript 整合

### TypeScript 6

專案使用 TypeScript ~6.0.2，這是 TypeScript 的最新主要版本，在型別推導和檢查速度上都有顯著提升。

### Interface vs Type

Gitpage 在 `api.ts` 中使用 `interface` 定義所有 API 的資料結構：

```typescript
export interface User {
    id: number
    username: string
    bio: string
    avatar_url: string
    created_at: string
}

export interface Repo {
    id: number
    user_id: number
    username?: string
    name: string
    description: string
    is_private: boolean
    default_branch: string
    owner_type?: string
    org_id?: number | null
    stars_count: number
    forks_count: number
    watch_count: number
    created_at: string
    updated_at: string
    forked_from?: number | null
}
```

選擇 `interface` 而非 `type` 的原因：
1. `interface` 的合併聲明（declaration merging）在擴充第三方型別時更友善
2. `interface` 的錯誤訊息通常更清晰
3. 對物件形狀的描述，`interface` 是慣用寫法

### 泛型在 API 層的應用

核心的 `request<T>` 函式是泛型在 Gitpage 中最典型的應用：

```typescript
async function request<T>(
    method: string,
    path: string,
    body?: unknown
): Promise<T> {
    const res = await fetch(`${BASE}${path}`, {
        method,
        headers: { ... },
        body: body ? JSON.stringify(body) : undefined,
    })
    if (!res.ok) { /* 錯誤處理 */ }
    return res.json()  // 回傳型別為 T
}
```

每個 API 函式都利用 TypeScript 的型別推導：

```typescript
// 編譯器會自動推導 listRepos() 的回傳型別
export function listRepos() {
    return request<{ repos: Repo[] }>('GET', '/api/repos')
}

// 錯誤的欄位存取會被編譯器攔截
const data = await listRepos()
console.log(data.repos)      // ✅ OK
console.log(data.repo)       // ❌ TypeScript 錯誤：不存在 'repo'
```

### 元件 Props 的型別定義

```typescript
// Layout.tsx
export default function Layout({ children }: { children: React.ReactNode }) { ... }

// MarkdownView.tsx
export default function MarkdownView({ html }: { html: string }) { ... }

// Spinner.tsx
export default function Spinner({ size }: { size?: number }) { ... }

// Pagination.tsx
interface PaginationProps {
    currentPage: number
    totalPages: number
    onPageChange: (page: number) => void
}
export default function Pagination({ currentPage, totalPages, onPageChange }: PaginationProps) { ... }
```

## 無狀態管理函式庫的策略

### 為什麼不使用 Redux / Zustand / Jotai

Gitpage 選擇不引入任何狀態管理函式庫，原因如下：

1. **狀態的範圍有限**：絕大多數狀態都是頁面級別的（當前頁面的 repo 列表、檔案內容等），不需要跨頁面共享。
2. **認證狀態的管理**：使用傳統的 `localStorage` + `getToken()` 模式。Token 在每個 API 請求時被注入（而不是存在全域 store 中）。
3. **減少依賴**：三個 runtime 依賴（react, react-dom, react-router-dom）意味著更小的 bundle 體積和更少的升級煩惱。
4. **直接明瞭**：`useState` + `useEffect` 模式對於頁面級資料流來說足夠表達。

### 跨頁面狀態的處理

對於少數需要跨頁面的狀態（如登入狀態），使用模組級函式而非 Context：

```typescript
// api.ts
function getToken(): string | null {
    return localStorage.getItem('token')
}

export function isLoggedIn(): boolean {
    return !!getToken()
}

// Layout.tsx — 直接在 render 中檢查
const loggedIn = isLoggedIn()
```

這種模式的優點是簡單且可測試：不需要 Provider 包裹、不需要 useContext、狀態變化通過頁面刷新自然反映。

## 頁面級狀態管理模式

### 典型的頁面元件模式

```tsx
// RepoPage.tsx — 典型的資料獲取頁面
export default function RepoPage() {
    const { id } = useParams<{ id: string }>()
    // 1. 多個 useState 管理不同資料
    const [repo, setRepo] = useState<Repo | null>(null)
    const [entries, setEntries] = useState<TreeEntry[]>([])
    const [readmeHtml, setReadmeHtml] = useState('')
    const [commits, setCommits] = useState<CommitInfo[]>([])
    const [loading, setLoading] = useState(true)
    const [err, setErr] = useState('')
    const [username, setUsername] = useState('')
    const [starred, setStarred] = useState(false)

    // 2. useEffect 觸發初始載入
    useEffect(() => {
        if (!id) return
        setLoading(true)
        getRepo(numId).then(async r => {
            setRepo(r.repo)
            const [treeRes, readmeRes, commitRes] = await Promise.all([
                listTree(uname, r.repo.name, r.repo.default_branch),
                getReadme(uname, r.repo.name, r.repo.default_branch),
                listCommits(uname, r.repo.name, r.repo.default_branch),
            ])
            setEntries(treeRes.entries)
            if (readmeRes.has_readme && readmeRes.rendered)
                setReadmeHtml(readmeRes.rendered)
            setCommits(commitRes.commits)
        }).catch(e => setErr(e.message))
          .finally(() => setLoading(false))
    }, [id])

    // 3. 條件渲染：載入中 → 錯誤 → 正常內容
    if (loading) return <Spinner />
    if (err) return <div className="error">{err}</div>
    if (!repo) return <div>儲存庫不存在</div>

    return ( /* JSX */ )
}
```

### 為什麼不將多個 useState 合併為一個物件

雖然可以寫成：

```typescript
const [state, setState] = useState({
    repo: null as Repo | null,
    entries: [] as TreeEntry[],
    loading: true,
    err: '',
})
```

但 Gitpage 選擇分散的 `useState`，因為：
1. 合併更新時不需要手動 spread
2. 單一 `setState` 不會造成其他無關欄位的重新渲染
3. 程式碼更容易閱讀和重構

## CSS 方案

### 純 CSS 的選擇

Gitpage 使用單一的 `index.css` 檔案（約 426 行），不使用 Tailwind、CSS Modules、styled-components 或任何 CSS-in-JS 方案。

```css
/* ── Reset & Base ── */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
html { font-size: 16px; }
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, ...;
    background: #fff;
    color: #111;
}
```

CSS 變數用於保持一致性：

```css
:root {
    --max-w: 640px;
    --border: 1px solid #e5e5e5;
    --border-focus: #000;
    --muted: #7c7c7c;
    --radius: 6px;
    --input-bg: #fafafa;
    --btn-radius: 8px;
}
```

### 自訂類別前綴

使用 `.markdown-body`、`.topnav`、`.bottom-nav`、`.main-content` 等簡單前綴，避免命名衝突。不採用 BEM 等方式，維持最低限度的 CSS 組織。

## 中文 UI 設計

### 語言策略

所有使用者面向的字串使用繁體中文：

```tsx
// FileEditorPage.tsx
if (!repo) return <div>載入中...</div>
if (err) return <div className="error">{err}</div>
```

```tsx
// 後端錯誤訊息
Err(AppError::NotFound("儲存庫不存在"))
```

### 佈局設計

- **頂部導航**：Logo + 頁面連結
- **底部導航**：行動裝置友善的固定工具列
- **內容容器**：最大寬度 640px 的居中佈局
- **表單元素**：大 padding（14px 16px），適合觸控操作

## Vite 建置工具

### 為何選擇 Vite

Vite 8 提供：
1. **極快的 HMR**（Hot Module Replacement）：開發中修改程式碼，幾乎即時反映在瀏覽器上
2. **ES modules 原生支援**：開發階段直接使用瀏覽器原生 ES module，不需要打包
3. **TypeScript 原生處理**：Vite 使用 esbuild 進行 TypeScript 轉譯，跳過 tsc
4. **Rollup 生產打包**：生產建置使用 Rollup，產生優化過的靜態資源

### 建置腳本

```json
{
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
}
```

- `tsc -b`：TypeScript 型別檢查（不產生輸出）
- `vite build`：Rollup 打包，輸出至 `dist/` 目錄

### 生產建置的輸出

```
frontend/dist/
├── index.html
├── assets/
│   ├── index-xxxxx.js     — 打包後的 JS
│   └── index-xxxxx.css    — 抽取的 CSS
└── vite.svg
```

生產環境中，這些靜態檔案由 Axum 的 fallback handler 提供（見 `_wiki/spa-fallback.md`）。

## 前端專案結構

```
frontend/
├── index.html              — Vite 入口 HTML
├── package.json            — 依賴管理
├── vite.config.ts          — Vite 設定（含 dev proxy）
├── tsconfig.json           — TypeScript 設定
├── eslint.config.js        — ESLint 設定
├── public/
│   └── vite.svg
└── src/
    ├── main.tsx            — React 入口
    ├── App.tsx             — 路由定義（37 個路由）
    ├── api.ts              — 所有 API 請求（707 行）
    ├── index.css           — 全域樣式（426 行）
    ├── components/
    │   ├── Layout.tsx      — 頂部 + 底部導航佈局
    │   ├── MarkdownView.tsx — Markdown 渲染（含 KaTeX/Mermaid）
    │   ├── Spinner.tsx     — 載入動畫
    │   └── Pagination.tsx  — 分頁元件
    └── pages/
        ├── RepoPage.tsx    — 儲存庫首頁
        ├── FileViewPage.tsx — 檔案檢視
        ├── FileExplorerPage.tsx — 檔案瀏覽器
        ├── CommitsPage.tsx — Commit 歷史
        └── ... (33 個頁面元件)
```

## 路由系統

使用 `react-router-dom` v7，在 `App.tsx` 中集中定義所有路由：

```tsx
<BrowserRouter>
    <Layout>
        <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/repo/:id" element={<RepoPage />} />
            <Route path="/u/:username" element={<UserProfilePage />} />
            <Route path="/org/:name" element={<OrgDetail />} />
            <Route path="*" element={<Navigate to="/" replace />} />
            {/* ... 共 37 個路由 */}
        </Routes>
    </Layout>
</BrowserRouter>
```

所有的頁面元件透過 `<Layout>` 包裹，獲得統一的頂部和底部導航。

## 參考資料

- `frontend/package.json` — 前端依賴（React 19, react-router-dom 7, Vite 8, TypeScript 6）
- `frontend/src/App.tsx` — 37 個路由定義
- `frontend/src/api.ts` — 統一的 API 請求層（`request<T>()`）
- `frontend/src/components/Layout.tsx` — 佈局元件
- `frontend/src/pages/RepoPage.tsx` — 典型頁面元件模式
- `frontend/src/index.css` — 426 行純 CSS 樣式
- `frontend/vite.config.ts` — Vite 設定（dev proxy）
- `_wiki/spa-fallback.md` — 生產環境的 SPA 路由後備機制
