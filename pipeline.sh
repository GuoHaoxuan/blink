#!/bin/bash

export PATH=/afs/ihep.ac.cn/soft/common/sysgroup/hep_job/bin:$PATH
cd /scratchfs/gecam/guohx/pipeline

# 检查上一次任务是不是还没计算完
if [ $(hep_q -u | wc -l) -ne 4 ]; then
  echo "Previous jobs are still running. Exiting."
  exit 0
fi

if [ $(ls | grep "work." | wc -l) -ne 0 ]; then
  echo "Previous Task: Calculation."
  comm -23 \
    <(find . \( -name "work.out.*" -or -name "work.err.*" \) -and -size 0 | sort) \
    <(hep_q -u | grep work.py | awk '{ print "./work.py.out." $1 "\n./work.py.err." $1 }' | sort) \
    | xargs rm -rf

  hep_sub -mem 8192 -g hxmt render

else
  echo "Previous Task: Rendering."
  comm -23 \
    <(find . \( -name "render.out.*" -or -name "render.err.*" \) -and -size 0 | sort) \
    <(hep_q -u | grep render.py | awk '{ print "./render.py.out." $1 "\n./render.py.err." $1 }' | sort) \
    | xargs rm -rf

  sqlite3 blink.db < sql/refresh_task.sql
  hep_sub -mem 8192 -g hxmt work
fi
