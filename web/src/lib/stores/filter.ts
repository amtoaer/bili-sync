import { writable } from 'svelte/store';

export interface AppState {
	query: string;
	currentPage: number;
	videoSource: {
		type: string;
		id: string;
	} | null;
}

export const appStateStore = writable<AppState>({
	query: '',
	currentPage: 0,
	videoSource: null
});

export const ToQuery = (state: AppState): string => {
	const { query, videoSource } = state;
	const params = new URLSearchParams();
	if (state.currentPage > 0) {
		params.set('page', String(state.currentPage));
	}
	if (query.trim()) {
		params.set('query', query);
	}
	if (videoSource && videoSource.type && videoSource.id) {
		params.set(videoSource.type, videoSource.id);
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
} => {
	const params: {
		query?: string;
		collection?: number;
		favorite?: number;
		submission?: number;
		watch_later?: number;
	} = {};

	if (state.query.trim()) {
		params.query = state.query;
	}

	if (state.videoSource && state.videoSource.type && state.videoSource.id) {
		const { type, id } = state.videoSource;
		params[type as 'collection' | 'favorite' | 'submission' | 'watch_later'] = parseInt(id);
	}

	return params;
};

// 检查是否有活动的筛选条件
export const hasActiveFilters = (state: AppState): boolean => {
	return !!(state.query.trim() || state.videoSource);
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

export const resetCurrentPage = () => {
	appStateStore.update((state) => ({
		...state,
		currentPage: 0
	}));
};

export const setAll = (
	query: string,
	currentPage: number,
	videoSource: { type: string; id: string } | null
) => {
	appStateStore.set({
		query,
		currentPage,
		videoSource
	});
};

export const clearAll = () => {
	appStateStore.set({
		query: '',
		currentPage: 0,
		videoSource: null
	});
};
