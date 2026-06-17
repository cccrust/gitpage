# Docker 設計問答

## Q：所有使用者共用一個容器，還是每人各自一個？

目前：**共用一個容器**（gitpage server + sshd + dev tools）。

未來可走向每人/每組織一台獨立容器，VPS 風格的隔離。

## Q：要引入 K8s 嗎？

不需要。K8s 為大型叢集設計（多台機器、數百容器），對單機或幾台機器太重。

替代方案：用 Docker SDK（`bollard` crate）直接在 axum 內管理容器生命週期，輕量、好維護。

## Q：各自獨立容器會不會有 port 衝突？

不會。每個容器有自己的 network namespace，alice 的 port 3000 和 bob 的 port 3000 完全隔離。

```
請求進 → gitpage proxy (host:8080)
         ├→ /app/alice/hello-rust/*  → docker IP 172.17.0.3:3000
         ├→ /app/bob/my-app/*        → docker IP 172.17.0.4:3000
         └→ /pages/alice/blog/*      → 直接讀靜態檔案
```

同 port 3000，不同容器，Docker bridge network 隔開，不衝突。

## Q：Docker 內部 IP（172.17.0.x）外部連得到嗎？

連不到，也不需要。外部只看到：

```
外部機器 → http://host:8080/app/alice/hello-rust/
                        ↓
              gitpage proxy (host port 8080)
                        ↓
              docker inspect 解析容器 IP
                        ↓
              172.17.0.3:3000 (宿主機內部 bridge)
```

Docker 容器 IP 只存在宿主機 kernel 的 bridge network 裡。對外一律透過 host port 8080 的 reverse proxy，IP 路由對客戶端完全透明。

## Q：為什麼不用 docker-compose？

docker-compose 適合固定多容器拓撲（web + db + redis），gitpage 需要的是一台 host 動態管理 N 個使用者容器，用程式碼（Docker SDK）控制比 yaml 靈活。

## Q：SSH 怎麼處理？

共用容器模式：一個 sshd 服務所有使用者，各自帳號密碼。

獨立容器模式（未來）：每人一台容器 + 各自 sshd，宿主機分配 port（如 22001→alice, 22002→bob）。使用者 `ssh -p 22001 alice@host`。

## Q：gitpage-agent 是什麼？

未來規劃：容器內常駐輕量 agent，接收宿主機指令：

- `deploy <user> <repo>` — clone bare repo → 安裝相依 → 建置 → 啟動
- `status` — 回報專案執行狀態
- `stop` — 停止專案

不必等使用者 SSH 登入，Git push 就可自動觸發建置。

## Q：現在 image 有幾層？

- `gitpage-dev-base:latest` — 開發工具基底（~3.4GB，含 rustup+rust, uv+python, node, opencode）
- `gitpage:latest` — app image（~570MB，FROM base + binary + config）

工具鏈更新時只需重建 base，app image 秒級重建。
