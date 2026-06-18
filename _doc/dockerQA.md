# Docker 設計問答

## Q：所有使用者共用一個容器，還是每人各自一個？

**v1.2 起：每人各自獨立容器。**

使用者註冊時自動建立 `gitpage-{username}` 容器，以 `sleep infinity` 保持運行。應用部署透過 `docker exec` 在容器內執行，完全不共享。

## Q：要引入 K8s 嗎？

不需要。K8s 為大型叢集設計（多台機器、數百容器），對單機或幾台機器太重。

替代方案：用 Docker SDK（`bollard` crate）直接在 axum 內管理容器生命週期，輕量、好維護。

## Q：各自獨立容器會不會有 port 衝突？

不會。每個容器有自己的 network namespace，alice 的 port 4000 和 bob 的 port 4000 完全隔離。

```
請求進 → gitpage proxy (host:8080)
         ├→ /app/alice/hello-rust/*  → 172.17.0.2:4000
         ├→ /app/bob/my-app/*        → 172.17.0.3:4000
         └→ /pages/alice/blog/*      → 直接讀靜態檔案
```

同 port 4000，不同容器，Docker bridge network 隔開，不衝突。

## Q：Docker 內部 IP（172.17.0.x）外部連得到嗎？

連不到，也不需要。外部只看到：

```
外部機器 → http://host:8080/app/alice/hello-rust/
                        ↓
              gitpage proxy (host port 8080)
                        ↓
              docker inspect 解析容器 IP
                        ↓
              172.17.0.2:4000 (宿主機內部 bridge)
```

Docker 容器 IP 只存在宿主機 kernel 的 bridge network 裡。對外一律透過 host port 8080 的 reverse proxy，IP 路由對客戶端完全透明。

## Q：為什麼不用 docker-compose？

docker-compose 適合固定多容器拓撲（web + db + redis），gitpage 需要的是一台 host 動態管理 N 個使用者容器，用程式碼（Docker SDK）控制比 yaml 靈活。

## Q：SSH 怎麼處理？

目前每人容器內有 `openssh-server`，port 22/tcp 已 exposed（Docker 分配動態 host port）。

規劃中：宿主機分配固定 port（如 22001→alice, 22002→bob），使用者 `ssh -p 22001 alice@host` 直接進入容器。

## Q：gitpage-agent 是什麼？

原規劃在容器內常駐輕量 agent 接收指令。v1.2 改用 **Docker exec** 直接執行：

- `exec_build(user, repo, cmd)` → `docker exec gitpage-{user} sh -c "cd /workspace/{repo}/source && {cmd}"`
- `exec_start_detached(user, repo, cmd, port, env)` → 背景啟動 app（detached exec）
- `exec_check_status(user, repo, port)` → `lsof -i :port` 檢查
- `exec_stop_app(user, port)` → `lsof -ti :port | xargs kill -9`

不必 agent，不必 SSH 登入，Git push 就可自動觸發建置。

## Q：現在 image 有幾層？

- `gitpage-dev-base:latest` — 開發工具基底（~3.4GB，含 rustup+rust, uv+python, node, opencode, openssh-server）
- `gitpage:latest` — app image（~570MB，FROM base + binary + config，用於 `run_docker.sh` 共用容器模式）

v1.2 的每人獨立容器使用 `gitpage-dev-base:latest` 直接啟動（+`sleep infinity` CMD），不再需要 `gitpage:latest`。
