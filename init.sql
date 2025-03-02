CREATE TABLE
    tasks (
        created_at TEXT NOT NULL, -- 创建时间
        updated_at TEXT NOT NULL, -- 修改时间
        retry_times INTEGER NOT NULL, -- 重试次数
        worker TEXT NOT NULL, -- 处理者
        status TEXT NOT NULL, -- 状态 Pending, Running, Finished, Failed
        error TEXT NOT NULL, -- 错误信息
        time TEXT NOT NULL, -- 要处理的时间
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        UNIQUE (time, satellite, detector) ON CONFLICT IGNORE
    );

CREATE TABLE
    IF NOT EXISTS signals (
        start TEXT NOT NULL, -- 开始时间
        stop TEXT NOT NULL, -- 结束时间
        events TEXT NOT NULL, -- 事件
        position TEXT NOT NULL, -- 位置
        lightnings TEXT NOT NULL -- 闪电
    );
