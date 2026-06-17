# docker build -t gitpage-dev-base:latest -f Dockerfile.base .
# 這會下載 Python (~50MB) 和 Rust toolchain (~500MB)，第一次大概要 5-15 分鐘。跑完後再：
# docker build -t gitpage:latest .
# 主 build 就只拷貝 binary + config，幾秒鐘就好。最後 ./run_docker.sh 啟動容器。
set -x
# 第一次（慢，下載 Python + Rust）
docker build -t gitpage-dev-base:latest -f Dockerfile.base .
./run_docker.sh --build  # 重新 build app image（不重 build base）

# 以後每次（快，幾秒）
#./run_docker.sh          # 用現有 image
