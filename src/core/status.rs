use crate::Result;

static STATUS_MAX_RETRY: u32 = 0b100;
static STATUS_OK: u32 = 0b111;

/// 用来表示下载的状态，不想写太多列了，所以仅使用一个 u32 表示。
/// 从低位开始，固定每三位表示一种数据的状态，从 0b000 开始，每失败一次加一，最多 0b100（即重试 4 次），
/// 如果成功，将对应的三位设置为 0b111。
/// 当所有任务都成功或者由于尝试次数过多失败，为 status 最高位打上标记 1，将来不再继续尝试。
pub struct Status(u32);

impl Status {
    /// 如果 status 整体大于等于 1 << 31，则表示任务已经被处理过，不再需要重试。
    /// 数据库可以使用 status < Status::handled() 来筛选需要处理的内容。
    pub fn handled() -> u32 {
        1 << 31
    }

    fn new(status: u32) -> Self {
        Self(status)
    }

    /// 从低到高检查状态，如果该位置的任务应该继续尝试执行，则返回 true，否则返回 false
    fn should_run(&self, size: usize) -> Vec<bool> {
        assert!(size < 10, "u32 can only store 10 status");
        (0..size).map(|x| self.check_continue(x)).collect()
    }

    /// 一般仅需要被内部调用
    fn set_flag(&mut self, handled: bool) {
        if handled {
            self.0 |= 1 << 31;
        } else {
            self.0 &= !(1 << 31);
        }
    }

    fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() < 10, "u32 can only store 10 status");
        for (i, res) in result.iter().enumerate() {
            self.set_result(res, i);
        }
        if self.should_run(result.len()).iter().all(|x| !x) {
            // 所有任务都成功或者由于尝试次数过多失败，为 status 最高位打上标记，将来不再重试
            self.set_flag(true)
        }
    }

    fn check_continue(&self, offset: usize) -> bool {
        assert!(offset < 10, "u32 can only store 10 status");
        let helper = !0u32;
        let sub_status = self.0 & (helper << (offset * 3)) & (helper >> (32 - 3 * offset - 3));
        sub_status < STATUS_MAX_RETRY
    }

    fn set_result(&mut self, result: &Result<()>, offset: usize) {
        if result.is_ok() {
            self.set_ok(offset);
        } else {
            self.plus_one(offset);
        }
    }

    fn plus_one(&mut self, offset: usize) {
        self.0 += 1 << (3 * offset);
    }

    fn set_ok(&mut self, offset: usize) {
        self.0 |= STATUS_OK << (3 * offset);
    }
}

impl From<Status> for u32 {
    fn from(status: Status) -> Self {
        status.0
    }
}

/// 从前到后分别表示：视频封面、分页下载、视频信息
pub struct VideoStatus(Status);

impl VideoStatus {
    pub fn new(status: u32) -> Self {
        Self(Status::new(status))
    }

    pub fn should_run(&self) -> Vec<bool> {
        self.0.should_run(3)
    }

    pub fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() == 3, "VideoStatus should have 3 status");
        self.0.update_status(result)
    }
}

impl From<VideoStatus> for u32 {
    fn from(status: VideoStatus) -> Self {
        status.0.into()
    }
}

/// 从前到后分别表示：视频封面、视频内容、视频信息
pub struct PageStatus(Status);

impl PageStatus {
    pub fn new(status: u32) -> Self {
        Self(Status::new(status))
    }

    pub fn should_run(&self) -> Vec<bool> {
        self.0.should_run(3)
    }

    pub fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() == 3, "PageStatus should have 3 status");
        self.0.update_status(result)
    }
}

impl From<PageStatus> for u32 {
    fn from(status: PageStatus) -> Self {
        status.0.into()
    }
}
