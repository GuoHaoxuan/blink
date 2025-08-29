#!/bin/bash

export PATH=/afs/ihep.ac.cn/soft/common/sysgroup/hep_job/bin:$PATH
cd /scratchfs/gecam/guohx/pipeline

# 检查上一次任务是不是还没计算完
declare -i current_jobs=$(hep_q -u | wc -l)
if [ $current_jobs -ne 4 ]; then
  echo "Previous jobs are still running. Exiting."
  exit 0
fi

# 清理上次的日志
comm -23 \
  <(find . \( -name "astro_sift.py.out.*" -or -name "astro_sift.py.err.*" \) -and -size 0 | sort) \
  <(hep_q -u | grep astro_sift.py | awk '{ print "./astro_sift.py.out." $1 "\n./astro_sift.py.err." $1 }' | sort) \
  | xargs rm -rf

# 拉取最新的代码
# git pull

# 编译代码
# cargo build --release

# 准备数据库
# sqlite3 blink.db < sql/init.sql
sqlite3 blink.db < sql/refresh_task.sql

# 准备软链接
# ln -sf ./target/release/work ./work
# ln -sf ./target/release/render ./render

# 提交运行
hep_sub -mem 8192 -g hxmt work
