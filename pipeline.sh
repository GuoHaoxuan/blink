#!/bin/bash

# 0. 检查上一次任务是不是还没计算完
declare -i current_jobs=$(hep_q -u | wc -l)
if [ $current_jobs -ne 4 ]; then
  echo "Previous jobs are still running. Exiting."
  exit 0
fi

# 1. 拉取最新的代码
git pull

# 2. 编译代码
cargo build --release

# 3. 准备数据库
sqlite3 blink.db < sql/init.sql
sqlite3 blink.db < sql/refresh_task.sql

# 4. 准备软链接
ln -sf ../target/release/snapshot-stepping ./snapshot-stepping
