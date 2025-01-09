CREATE TABLE
    IF NOT EXISTS tasks (
        created_at TEXT NOT NULL, -- 创建时间
        updated_at TEXT NOT NULL, -- 修改时间
        retry_times INTEGER NOT NULL, -- 重试次数
        lock_id TEXT NOT NULL, -- 为了加行锁的标志，如 hostname-pid
        time_hour TEXT NOT NULL, -- 消息内容
        satellite TEXT NOT NULL -- 消息内容
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
        average REAL NOT NULL, -- 平均值
    );
