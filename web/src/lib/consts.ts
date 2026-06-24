import { ClockIcon, FolderIcon, HeartIcon, UserIcon } from '@lucide/svelte/icons';

export const VIDEO_SOURCES = {
	FAVORITE: { type: 'favorite', title: '收藏夹', icon: HeartIcon },
	COLLECTION: { type: 'collection', title: '合集 / 列表', icon: FolderIcon },
	SUBMISSION: { type: 'submission', title: '用户投稿', icon: UserIcon },
	WATCH_LATER: { type: 'watch_later', title: '稍后再看', icon: ClockIcon }
};

/**
 * 弹幕同步阶段标签映射，对应 Rust 端 `danmaku_sync_generation` 字段。
 * 0=未开始；首次同步前不展示 badge。
 */
export const DANMAKU_GENERATION_LABELS: Record<
	number,
	{ text: string; variant: 'default' | 'secondary' | 'outline' | 'destructive' }
> = {
	0: { text: '待更新', variant: 'outline' },
	1: { text: '新鲜期', variant: 'default' },
	2: { text: '成熟期', variant: 'secondary' },
	3: { text: '老化期', variant: 'outline' },
	4: { text: '已冻结', variant: 'outline' }
};

/** 将任意可解析为日期的字符串格式化为相对时间（"2 小时前"）。 */
export function formatRelativeTime(input: string | Date | null | undefined): string {
	if (!input) return '从未同步';
	const then = typeof input === 'string' ? new Date(input.replace(' ', 'T') + 'Z') : input;
	const diff = Date.now() - then.getTime();
	if (Number.isNaN(diff)) return '从未同步';
	if (diff < 0) return '刚刚';
	const minutes = Math.floor(diff / 60_000);
	if (minutes < 1) return '刚刚';
	if (minutes < 60) return `${minutes} 分钟前`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours} 小时前`;
	const days = Math.floor(hours / 24);
	if (days < 30) return `${days} 天前`;
	const months = Math.floor(days / 30);
	if (months < 12) return `${months} 个月前`;
	const years = Math.floor(days / 365);
	return `${years} 年前`;
}
