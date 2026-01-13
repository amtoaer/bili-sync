import { writable } from 'svelte/store';

export interface AppState {
	query: string;
	currentPage: number;
	videoSource: {
		type: string;
		id: string;
	} | null;
	failedOnly: boolean;
}

export const appStateStore = writable<AppState>({
	query: '',
	currentPage: 0,
	videoSource: null,
	failedOnly: false
});

export const ToQuery = (state: AppState): string => {
	const { query, videoSource, currentPage, failedOnly } = state;
	const params = new URLSearchParams();
	if (currentPage > 0) {
		params.set('page', String(currentPage));
	}
	if (query.trim()) {
		params.set('query', query);
	}
	if (videoSource && videoSource.type && videoSource.id) {
		params.set(videoSource.type, videoSource.id);
	}
	if (failedOnly) {
		params.set('failed_only', 'true');
	}
	const queryString = params.toString();
	return queryString ? `videos?${queryString}` : 'videos';
};

// 将 AppState 转换为请求体中的筛选参数
export const ToFilterParams = (
	state: AppState
): {
	query?: string;
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	failed_only?: boolean;
} => {
	const params: {
		query?: string;
		collection?: number;
		favorite?: number;
		submission?: number;
		watch_later?: number;
		failed_only?: boolean;
	} = {};

	if (state.query.trim()) {
		params.query = state.query;
	}

	if (state.videoSource && state.videoSource.type && state.videoSource.id) {
		const { type, id } = state.videoSource;
		params[type as 'collection' | 'favorite' | 'submission' | 'watch_later'] = parseInt(id);
	}
	if (state.failedOnly) {
		params.failed_only = true;
	}
	return params;
};

// 检查是否有活动的筛选条件
export const hasActiveFilters = (state: AppState): boolean => {
	return !!(state.query.trim() || state.videoSource || state.failedOnly);
};

export const setQuery = (query: string) => {
	appStateStore.update((state) => ({
		...state,
		query
	}));
};

export const setVideoSourceFilter = (filter: { type: string; id: string }) => {
	appStateStore.update((state) => ({
		...state,
		videoSource: filter
	}));
};

export const clearVideoSourceFilter = () => {
	appStateStore.update((state) => ({
		...state,
		videoSource: null
	}));
};

export const setCurrentPage = (page: number) => {
	appStateStore.update((state) => ({
		...state,
		currentPage: page
	}));
};

export const setFailedOnly = (failedOnly: boolean) => {
	appStateStore.update((state) => ({
		...state,
		failedOnly
	}));
};

export const resetCurrentPage = () => {
	appStateStore.update((state) => ({
		...state,
		currentPage: 0
	}));
};

export const setAll = (
	query: string,
	currentPage: number,
	videoSource: { type: string; id: string } | null,
	failedOnly: boolean
) => {
	appStateStore.set({
		query,
		currentPage,
		videoSource,
		failedOnly
	});
};

export const clearAll = () => {
	appStateStore.set({
		query: '',
		currentPage: 0,
		videoSource: null,
		failedOnly: false
	});
};
