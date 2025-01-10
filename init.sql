CREATE TABLE
    tasks (
        created_at TEXT NOT NULL, -- 创建时间
        updated_at TEXT NOT NULL, -- 修改时间
        retry_times INTEGER NOT NULL, -- 重试次数
        worker TEXT NOT NULL, -- 处理者
        status TEXT NOT NULL, -- 状态 Pending, Running, Finished, Failed
        time TEXT NOT NULL, -- 要处理的时间
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        UNIQUE (time, satellite, detector) ON CONFLICT IGNORE
    );

CREATE TABLE
    IF NOT EXISTS records (
        start TEXT NOT NULL, -- 开始时间
        stop TEXT NOT NULL, -- 结束时间
        bin_size_min INTEGER NOT NULL,
        bin_size_max INTEGER NOT NULL,
        bin_size_best INTEGER NOT NULL,
        delay INTEGER NOT NULL,
        count INTEGER NOT NULL, -- 计数
        average REAL NOT NULL -- 平均值
    );
