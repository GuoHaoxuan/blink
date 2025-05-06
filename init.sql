CREATE TABLE
    task (
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
    IF NOT EXISTS signal (
        start TEXT NOT NULL, -- 开始时间
        stop TEXT NOT NULL, -- 结束时间
        best_start TEXT NOT NULL, -- 最佳开始时间
        best_stop TEXT NOT NULL, -- 最佳结束时间
        fp_year REAL NOT NULL, -- 年误触发个数
        count INTEGER NOT NULL, -- 事件个数
        best_count INTEGER NOT NULL, -- 最佳事件个数
        background REAL NOT NULL, -- 每秒本底
        events TEXT NOT NULL, -- 事件
        light_curve_1s TEXT NOT NULL, -- 光变曲线
        light_curve_1s_filtered TEXT NOT NULL, -- 有效事件光变曲线
        light_curve_100ms TEXT NOT NULL, -- 100ms 光变曲线
        light_curve_100ms_filtered TEXT NOT NULL, -- 有效事件 100ms 光变曲线
        longitude REAL NOT NULL, -- 经度
        latitude REAL NOT NULL, -- 纬度
        altitude REAL NOT NULL, -- 高度
        q1 REAL NOT NULL, -- Q1
        q2 REAL NOT NULL, -- Q2
        q3 REAL NOT NULL, -- Q3
        orbit TEXT NOT NULL, -- 轨道
        lightnings TEXT NOT NULL, -- 闪电
        associated_lightning_count INTEGER NOT NULL, -- 关联闪电个数
        coincidence_probability REAL NOT NULL, -- 偶然符合概率
        satellite TEXT NOT NULL, -- 要处理的卫星
        detector TEXT NOT NULL, -- 要处理的探测器
        UNIQUE (start, satellite, detector) ON CONFLICT IGNORE
    );
