use anyhow::Result;

static STATUS_MAX_RETRY: u32 = 0b100;
static STATUS_OK: u32 = 0b111;

/// 用来表示下载的状态，不想写太多列了，所以仅使用一个 u32 表示。
/// 从低位开始，固定每三位表示一种数据的状态，从 0b000 开始，每失败一次加一，最多 0b100（即重试 4 次），
/// 如果成功，将对应的三位设置为 0b111。
/// 当所有任务都成功或者由于尝试次数过多失败，为 status 最高位打上标记 1，将来不再继续尝试。
#[derive(Clone)]
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

    /// 一般仅需要被内部调用，用来设置最高位的标记
    fn set_flag(&mut self, handled: bool) {
        if handled {
            self.0 |= 1 << 31;
        } else {
            self.0 &= !(1 << 31);
        }
    }

    /// 从低到高检查状态，如果该位置的任务应该继续尝试执行，则返回 true，否则返回 false
    fn should_run(&self, size: usize) -> Vec<bool> {
        assert!(size < 10, "u32 can only store 10 status");
        (0..size).map(|x| self.check_continue(x)).collect()
    }

    /// 如果任务的执行次数小于 STATUS_MAX_RETRY，说明可以继续运行
    fn check_continue(&self, offset: usize) -> bool {
        assert!(offset < 10, "u32 can only store 10 status");
        self.get_status(offset) < STATUS_MAX_RETRY
    }

    /// 根据任务结果更新状态，如果任务成功，设置为 STATUS_OK，否则加一
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

    fn set_result(&mut self, result: &Result<()>, offset: usize) {
        if result.is_ok() {
            // 如果任务已经执行到最大次数，那么此时 Result 也是 Ok，此时不应该更新状态
            if self.get_status(offset) < STATUS_MAX_RETRY {
                self.set_ok(offset);
            }
        } else {
            self.plus_one(offset);
        }
    }

    /// 根据 mask 设置状态，如果 mask 为 false，则清除对应的状态
    fn set_mask(&mut self, mask: &[bool]) {
        assert!(mask.len() < 10, "u32 can only store 10 status");
        for (i, &m) in mask.iter().enumerate() {
            if !m {
                self.clear(i);
                self.set_flag(false);
            }
        }
    }

    fn plus_one(&mut self, offset: usize) {
        self.0 += 1 << (3 * offset);
    }

    fn set_ok(&mut self, offset: usize) {
        self.0 |= STATUS_OK << (3 * offset);
    }

    fn clear(&mut self, offset: usize) {
        self.0 &= !(STATUS_OK << (3 * offset));
    }

    fn get_status(&self, offset: usize) -> u32 {
        let helper = !0u32;
        (self.0 & (helper << (offset * 3)) & (helper >> (32 - 3 * offset - 3))) >> (offset * 3)
    }
}

impl From<Status> for u32 {
    fn from(status: Status) -> Self {
        status.0
    }
}

/// 从前到后分别表示：视频封面、视频信息、Up 主头像、Up 主信息、分 P 下载
#[derive(Clone)]
pub struct VideoStatus(Status);

impl VideoStatus {
    pub fn new(status: u32) -> Self {
        Self(Status::new(status))
    }

    pub fn set_mask(&mut self, clear: &[bool]) {
        assert!(clear.len() == 5, "VideoStatus should have 5 status");
        self.0.set_mask(clear)
    }

    pub fn should_run(&self) -> Vec<bool> {
        self.0.should_run(5)
    }

    pub fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() == 5, "VideoStatus should have 5 status");
        self.0.update_status(result)
    }
}

impl From<VideoStatus> for u32 {
    fn from(status: VideoStatus) -> Self {
        status.0.into()
    }
}

/// 从前到后分别表示：视频封面、视频内容、视频信息
#[derive(Clone)]
pub struct PageStatus(Status);

impl PageStatus {
    pub fn new(status: u32) -> Self {
        Self(Status::new(status))
    }

    pub fn set_mask(&mut self, clear: &[bool]) {
        assert!(clear.len() == 4, "PageStatus should have 4 status");
        self.0.set_mask(clear)
    }

    pub fn should_run(&self) -> Vec<bool> {
        self.0.should_run(4)
    }

    pub fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() == 4, "PageStatus should have 4 status");
        self.0.update_status(result)
    }
}

impl From<PageStatus> for u32 {
    fn from(status: PageStatus) -> Self {
        status.0.into()
    }
}

#[cfg(test)]
mod test {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn test_status() {
        let mut status = Status::new(0);
        assert_eq!(status.should_run(3), vec![true, true, true]);
        for count in 1..=3 {
            status.update_status(&[Err(anyhow!("")), Ok(()), Ok(())]);
            assert_eq!(status.should_run(3), vec![true, false, false]);
            assert_eq!(u32::from(status.clone()), 0b111_111_000 + count);
        }
        status.update_status(&[Err(anyhow!("")), Ok(()), Ok(())]);
        assert_eq!(status.should_run(3), vec![false, false, false]);
        assert_eq!(u32::from(status), 0b111_111_100 | Status::handled());
    }
}
