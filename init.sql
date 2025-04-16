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
        fp_year REAL NOT NULL, -- 年误触发个数
        longitude REAL NOT NULL, -- 经度
        latitude REAL NOT NULL, -- 纬度
        altitude REAL NOT NULL, -- 高度
        events TEXT NOT NULL, -- 事件
        lightnings TEXT NOT NULL -- 闪电
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        UNIQUE (start, satellite, detector) ON CONFLICT IGNORE
    );
