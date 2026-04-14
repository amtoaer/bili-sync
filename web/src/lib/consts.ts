import { ClockIcon, FolderIcon, HeartIcon, UserIcon } from '@lucide/svelte/icons';

export const VIDEO_SOURCES = {
	FAVORITE: { type: 'favorite', title: '收藏夹', icon: HeartIcon },
	COLLECTION: { type: 'collection', title: '合集 / 列表', icon: FolderIcon },
	SUBMISSION: { type: 'submission', title: '用户投稿', icon: UserIcon },
	WATCH_LATER: { type: 'watch_later', title: '稍后再看', icon: ClockIcon }
};
