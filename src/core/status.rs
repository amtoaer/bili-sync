use crate::Result;

static STATUS_MAX_RETRY: u32 = 0b100;
static STATUS_OK: u32 = 0b111;

/// 用来表示下载的状态，不想写太多列了，所以仅使用一个 u32 表示
/// 从低位开始，固定每三位表示一种数据的状态
pub struct Status(u32);

impl Status {
    pub fn new(status: u32) -> Self {
        Self(status)
    }

    pub fn should_run(&self) -> [bool; 4] {
        let mut result = [false; 4];
        for (i, res) in result.iter_mut().enumerate() {
            *res = self.check_continue(i);
        }
        result
    }

    pub fn update_status(&mut self, result: &[Result<()>]) {
        assert!(result.len() >= 4, "result length must be 4");
        for (i, res) in result.iter().enumerate().take(4) {
            self.set_result(res, i);
        }
        if self.should_run().iter().all(|x| !x) {
            // 所有任务都成功或者由于尝试次数过多失败，为 status 最高位打上标记，将来不再重试
            self.0 |= 1 << 31;
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
