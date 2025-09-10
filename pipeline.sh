#!/bin/bash

export PATH=/afs/ihep.ac.cn/soft/common/sysgroup/hep_job/bin:$PATH
cd /scratchfs/gecam/guohx/pipeline

# 检查上一次任务是不是还没计算完
if [ $(hep_q -u | wc -l) -ne 4 ]; then
  echo "Previous jobs are still running. Exiting."
  exit 0
fi

comm -23 \
    <(find . \( -name "pipeline_run.out.*" -or -name "pipeline_run.err.*" \) -and -size 0 | sort) \
    <(hep_q -u | grep pipeline_run.py | awk '{ print "./pipeline_run.py.out." $1 "\n./pipeline_run.py.err." $1 }' | sort) \
    | xargs rm -rf

hep_sub -mem 8192 -g hxmt pipeline_run.sh
