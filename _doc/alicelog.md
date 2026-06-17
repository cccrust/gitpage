(.venv) cccuser@cccimacdeiMac gitpage % ssh alice@localhost -p 2222
The authenticity of host '[localhost]:2222 ([::1]:2222)' can't be established.
ED25519 key fingerprint is SHA256:aj98fw3RIe4GmucV+rzI1MCi85b8XG+snHPT1+thjic.
This key is not known by any other names.
Are you sure you want to continue connecting (yes/no/[fingerprint])? yes
Warning: Permanently added '[localhost]:2222' (ED25519) to the list of known hosts.
alice@localhost's password: 
Permission denied, please try again.
alice@localhost's password: 
Linux 798b10afaaa8 6.12.54-linuxkit #1 SMP Tue Nov  4 21:21:47 UTC 2025 aarch64

The programs included with the Debian GNU/Linux system are free software;
the exact distribution terms for each program are described in the
individual files in /usr/share/doc/*/copyright.

Debian GNU/Linux comes with ABSOLUTELY NO WARRANTY, to the extent
permitted by applicable law.
alice@798b10afaaa8:~$ ls
staging
alice@798b10afaaa8:~$ pwd
/home/alice
alice@798b10afaaa8:~$ cd staging
alice@798b10afaaa8:~/staging$ ls
rusttest
alice@798b10afaaa8:~/staging$ cd rusttest
alice@798b10afaaa8:~/staging/rusttest$ ls


git clone ssh://git@host:port/alice/repo.git

## ccc: 我已經執行了 run_docker.sh ，這時 server 應該已經起來了，我希望能直接用 ssh 連進去。你需要再修改程式嗎？還是我可以直接下 ssh 指令連？

完成。重新執行 ./run_docker.sh 建構後，就可以用 SSH 連進 container：
ssh root@localhost -p 2222
# password: gitpage
新增的檔案：
- entrypoint.sh — container 啟動腳本：產生 SSH host key → 起 sshd → 執行 gitpage
run_docker.sh 預設把 container 的 port 22 映射到 host 的 2222（避免與 host 的 sshd 衝突），可透過 SSH_PORT 環境變數改掉。

## ccc: 如果我執行 run.sh ，我也希望能用 ssh 連進 alice 帳號，這該怎麼做？

alice 是 gitpage 的資料庫使用者，不是系統帳號。非 Docker 模式沒有隔離的 SSH server，直接用 host 的 SSH 連你自己的系統帳號即可。
run.sh 已補上 SSH 提示。執行 ./run.sh 後：
ssh cccuser@localhost   # 連自己的機器