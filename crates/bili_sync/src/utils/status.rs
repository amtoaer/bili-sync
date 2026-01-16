use std::marker::PhantomData;

use bili_sync_entity::{page, video};
use bili_sync_migration::{ExprTrait, IntoCondition};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, Condition};

use crate::error::ExecutionStatus;

pub static STATUS_NOT_STARTED: u32 = 0b000;
pub(super) static STATUS_MAX_RETRY: u32 = 0b100;
pub static STATUS_OK: u32 = 0b111;
pub static STATUS_COMPLETED: u32 = 1 << 31;

/// 用来表示下载的状态，不想写太多列了，所以仅使用一个 u32 表示。
/// 从低位开始，固定每三位表示一种子任务的状态。
/// 子任务状态从 0b000 开始，每执行失败一次将状态加一，最多 0b100（即允许重试 4 次），该值定义为 STATUS_MAX_RETRY。
/// 如果子任务执行成功，将状态设置为 0b111，该值定义为 STATUS_OK。
/// 子任务达到最大失败次数或者执行成功时，认为该子任务已经完成。
/// 当所有子任务都已经完成时，为最高位打上标记 1，表示整个下载任务已经完成。
#[derive(Clone, Copy)]
pub struct Status<const N: usize, C>(u32, PhantomData<C>);

impl<const N: usize, C> Default for Status<N, C> {
    fn default() -> Self {
        Self(0, PhantomData)
    }
}

impl<const N: usize, C> Status<N, C> {
    pub(crate) const LEN: usize = N;
    // 获取最高位的完成标记
    pub fn get_completed(&self) -> bool {
        self.0 >> 31 == 1
    }

    /// 依次检查所有子任务是否还应该继续执行，返回一个 bool 数组
    pub fn should_run(&self) -> [bool; N] {
        let mut result = [false; N];
        for (i, item) in result.iter_mut().enumerate() {
            *item = self.check_continue(i);
        }
        result
    }

    /// 重置所有失败的状态，将状态设置为 0b000，返回值表示 status 是否发生了变化
    pub fn reset_failed(&mut self) -> bool {
        let mut changed = false;
        for i in 0..N {
            let status = self.get_status(i);
            if status != STATUS_NOT_STARTED && status != STATUS_OK {
                self.set_status(i, STATUS_NOT_STARTED);
                changed = true;
            }
        }
        if changed {
            self.set_completed(false);
        }
        changed
    }

    /// 重置所有失败的状态，将状态设置为 0b000，返回值表示 status 是否发生了变化
    /// force 版本在普通版本的基础上，会额外检查是否存在需要运行的任务，如果存在则修正 completed 标记位为“未完成”
    /// 这个方法的典型用例是在引入新的任务状态后重置历史视频，允许历史视频执行新引入的任务
    pub fn force_reset_failed(&mut self) -> bool {
        let mut changed = self.reset_failed();
        // 理论上上面的 changed 就足够了，因为 completed 标志位的改变是由子任务状态的改变引起的，子任务没有改变则 completed 也不会改变
        // 但考虑特殊情况，新版本引入了一个新的子任务项，此时会出现明明有子任务未执行，但 completed 标记位仍然为 true 的情况
        // 当然可以在新版本迁移文件中全局重置 completed 标记位，但这样影响范围太大感觉不太好
        // 在后面进行这部分额外判断可以兼容这种情况，在由用户手动触发的 reset_failed 调用中修正 completed 标记位
        if !changed && self.get_completed() && self.should_run().into_iter().any(|x| x) {
            changed = true;
            self.set_completed(false);
        }
        changed
    }

    /// 覆盖某个子任务的状态
    pub fn set(&mut self, offset: usize, status: u32) {
        assert!(status < 0b1000, "status should be less than 0b1000");
        self.set_status(offset, status);
        if self.should_run().into_iter().all(|x| !x) {
            self.set_completed(true);
        } else {
            self.set_completed(false);
        }
    }

    /// 根据任务结果更新状态，任务结果是一个 Result 数组，需要与子任务一一对应
    /// 如果所有子任务都已经完成，那么打上最高位的完成标记
    pub fn update_status(&mut self, result: &[ExecutionStatus]) {
        assert!(result.len() == N, "result length should be equal to N");
        for (i, res) in result.iter().enumerate() {
            self.set_result(res, i);
        }
        if self.should_run().into_iter().all(|x| !x) {
            self.set_completed(true);
        } else {
            self.set_completed(false);
        }
    }

    /// 设置最高位的完成标记
    fn set_completed(&mut self, completed: bool) {
        if completed {
            self.0 |= 1 << 31;
        } else {
            self.0 &= !(1 << 31);
        }
    }

    /// 获取某个子任务的状态
    fn get_status(&self, offset: usize) -> u32 {
        (self.0 >> (offset * 3)) & 0b111
    }

    /// 设置某个子任务的状态
    fn set_status(&mut self, offset: usize, status: u32) {
        self.0 = (self.0 & !(0b111 << (offset * 3))) | (status << (offset * 3));
    }

    // 将某个子任务的状态加一（在任务失败时使用）
    fn plus_one(&mut self, offset: usize) {
        self.0 += 1 << (3 * offset);
    }

    // 设置某个子任务的状态为 STATUS_OK（在任务成功时使用）
    fn set_ok(&mut self, offset: usize) {
        self.0 |= STATUS_OK << (3 * offset);
    }

    /// 检查某个子任务是否还应该继续执行，实际是检查该子任务的状态是否小于 STATUS_MAX_RETRY
    fn check_continue(&self, offset: usize) -> bool {
        self.get_status(offset) < STATUS_MAX_RETRY
    }

    /// 根据子任务执行结果更新子任务的状态
    fn set_result(&mut self, result: &ExecutionStatus, offset: usize) {
        // 如果任务返回 Fixed 状态，那么无论之前的状态如何，都将状态设置为 Fixed 的状态
        if let ExecutionStatus::Fixed(status) = result {
            assert!(*status < 0b1000, "status should be less than 0b1000");
            self.set_status(offset, *status);
        } else if self.get_status(offset) < STATUS_MAX_RETRY {
            match result {
                ExecutionStatus::Succeeded | ExecutionStatus::Skipped => self.set_ok(offset),
                ExecutionStatus::Failed(_) => self.plus_one(offset),
                _ => {}
            }
        }
    }
}

impl<const N: usize, C> From<u32> for Status<N, C> {
    fn from(status: u32) -> Self {
        Status(status, PhantomData)
    }
}

impl<const N: usize, C> From<Status<N, C>> for u32 {
    fn from(status: Status<N, C>) -> Self {
        status.0
    }
}

impl<const N: usize, C> From<Status<N, C>> for [u32; N] {
    fn from(status: Status<N, C>) -> Self {
        let mut result = [0; N];
        for (i, item) in result.iter_mut().enumerate() {
            *item = status.get_status(i);
        }
        result
    }
}

impl<const N: usize, C> From<[u32; N]> for Status<N, C> {
    fn from(status: [u32; N]) -> Self {
        let mut result = Self::default();
        for (i, item) in status.iter().enumerate() {
            assert!(*item < 0b1000, "status should be less than 0b1000");
            result.set_status(i, *item);
        }
        if result.should_run().iter().all(|x| !x) {
            result.set_completed(true);
        }
        result
    }
}

/// 包含五个子任务，从前到后依次是：视频封面、视频信息、Up 主头像、Up 主信息、分页下载
pub type VideoStatus = Status<5, video::Column>;

impl VideoStatus {
    pub fn query_builder() -> StatusQueryBuilder<{ Self::LEN }, video::Column> {
        StatusQueryBuilder::new(video::Column::DownloadStatus)
    }
}

/// 包含五个子任务，从前到后分别是：视频封面、视频内容、视频信息、视频弹幕、视频字幕
pub type PageStatus = Status<5, page::Column>;

impl PageStatus {
    pub fn query_builder() -> StatusQueryBuilder<{ Self::LEN }, page::Column> {
        StatusQueryBuilder::new(page::Column::DownloadStatus)
    }
}

pub struct StatusQueryBuilder<const N: usize, C: ColumnTrait> {
    column: C,
}

impl<const N: usize, C: ColumnTrait> StatusQueryBuilder<N, C> {
    fn new(column: C) -> Self {
        Self { column }
    }

    /// 完成状态：所有子任务的状态都是成功
    pub fn succeeded(&self) -> Condition {
        let mut condition = Condition::all();
        for offset in 0..N as i32 {
            condition = condition.add(Expr::col(self.column).right_shift(offset * 3).bit_and(7).eq(7))
        }
        condition
    }

    /// 失败状态：存在任何失败的子任务
    pub fn failed(&self) -> Condition {
        let mut condition = Condition::any();
        for offset in 0..N as i32 {
            condition = condition.add(
                Expr::col(self.column)
                    .right_shift(offset * 3)
                    .bit_and(7)
                    .is_not_in([0, 7]),
            )
        }
        condition
    }

    /// 等待状态：所有子任务的状态都不是失败，且其中存在未开始
    pub fn waiting(&self) -> Condition {
        let mut condition = Condition::any();
        for offset in 0..N as i32 {
            condition = condition.add(Expr::col(self.column).right_shift(offset * 3).bit_and(7).eq(0))
        }
        condition.and(self.failed().not()).into_condition()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::*;

    #[test]
    fn test_status_update() {
        let mut status = Status::<3, video::Column>::default();
        assert_eq!(status.should_run(), [true, true, true]);
        for _ in 0..3 {
            status.update_status(&[
                ExecutionStatus::Failed(anyhow!("")),
                ExecutionStatus::Succeeded,
                ExecutionStatus::Succeeded,
            ]);
            assert_eq!(status.should_run(), [true, false, false]);
        }
        status.update_status(&[
            ExecutionStatus::Failed(anyhow!("")),
            ExecutionStatus::Succeeded,
            ExecutionStatus::Succeeded,
        ]);
        assert_eq!(status.should_run(), [false, false, false]);
        assert!(status.get_completed());
        status.update_status(&[
            ExecutionStatus::Fixed(1),
            ExecutionStatus::Fixed(4),
            ExecutionStatus::Fixed(7),
        ]);
        assert_eq!(status.should_run(), [true, false, false]);
        assert!(!status.get_completed());
        assert_eq!(<[u32; 3]>::from(status), [1, 4, 7]);
    }

    #[test]
    fn test_status_convert() {
        let testcases = [[0, 0, 1], [1, 2, 3], [3, 1, 2], [3, 0, 7]];
        for testcase in testcases.iter() {
            let status = Status::<3, video::Column>::from(testcase.clone());
            assert_eq!(<[u32; 3]>::from(status), *testcase);
        }
    }

    #[test]
    fn test_status_convert_and_update() {
        let testcases = [([0, 0, 1], [1, 7, 7]), ([3, 4, 3], [4, 4, 7]), ([3, 1, 7], [4, 7, 7])];
        for (before, after) in testcases.iter() {
            let mut status = Status::<3, video::Column>::from(before.clone());
            status.update_status(&[
                ExecutionStatus::Failed(anyhow!("")),
                ExecutionStatus::Succeeded,
                ExecutionStatus::Succeeded,
            ]);
            assert_eq!(<[u32; 3]>::from(status), *after);
        }
    }

    #[test]
    fn test_status_reset_failed() {
        // 重置一个出现部分失败但还有重试次数的任务，将所有的失败状态重置为 0
        let mut status = Status::<3, video::Column>::from([3, 4, 7]);
        assert!(!status.get_completed());
        assert!(status.reset_failed());
        assert!(!status.get_completed());
        assert_eq!(<[u32; 3]>::from(status), [0, 0, 7]);
        // 没有内容需要重置，但 completed 标记位是错误的（模拟新增一个子任务状态的情况）
        // 此时 reset_failed 不会修正 completed 标记位，而 force_reset_failed 会
        status.set_completed(true);
        assert!(status.get_completed());
        assert!(!status.reset_failed());
        assert!(status.get_completed());
        assert!(status.force_reset_failed());
        assert!(!status.get_completed());
        // 重置一个已经成功的任务，没有改变状态，也不会修改标记位
        let mut status = Status::<3, video::Column>::from([7, 7, 7]);
        assert!(status.get_completed());
        assert!(!status.reset_failed());
        assert!(status.get_completed());
        // 重置一个全部失败的任务，修改状态并且修改标记位
        let mut status = Status::<3, video::Column>::from([4, 4, 4]);
        assert!(status.get_completed());
        assert!(status.reset_failed());
        assert!(!status.get_completed());
        assert_eq!(<[u32; 3]>::from(status), [0, 0, 0]);
    }

    #[test]
    fn test_status_set() {
        // 设置子状态，从 completed 到 uncompleted
        let mut status = Status::<5, video::Column>::from([7, 7, 7, 7, 7]);
        assert!(status.get_completed());
        status.set(4, 0);
        assert!(!status.get_completed());
        assert_eq!(<[u32; 5]>::from(status), [7, 7, 7, 7, 0]);
        // 设置子状态，从 uncompleted 到 completed
        let mut status = Status::<5, video::Column>::from([4, 7, 7, 7, 0]);
        assert!(!status.get_completed());
        status.set(4, 7);
        assert!(status.get_completed());
        assert_eq!(<[u32; 5]>::from(status), [4, 7, 7, 7, 7]);
    }
}
