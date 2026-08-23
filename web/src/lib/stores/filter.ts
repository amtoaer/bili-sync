import { writable } from 'svelte/store';

export type StatusFilterValue = 'failed' | 'succeeded' | 'waiting' | null;
export type ValidationFilterValue = 'skipped' | 'invalid' | 'normal' | null;

export interface AppState {
	query: string;
	currentPage: number;
	videoSource: {
		type: string;
		id: string;
	} | null;
	statusFilter: StatusFilterValue | null;
	validationFilter: ValidationFilterValue | null;
	createdFrom: string | null;
	createdTo: string | null;
}

export const appStateStore = writable<AppState>({
	query: '',
	currentPage: 0,
	videoSource: null,
	statusFilter: null,
	validationFilter: 'normal',
	createdFrom: null,
	createdTo: null
});

export const ToQuery = (state: AppState): string => {
	const {
		query,
		videoSource,
		currentPage,
		statusFilter,
		validationFilter,
		createdFrom,
		createdTo
	} = state;
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
	if (statusFilter) {
		params.set('status_filter', statusFilter);
	}
	if (validationFilter) {
		params.set('validation_filter', validationFilter);
	}
	if (createdFrom) {
		params.set('created_from', createdFrom);
	}
	if (createdTo) {
		params.set('created_to', createdTo);
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
	status_filter?: Exclude<StatusFilterValue, null>;
	validation_filter?: Exclude<ValidationFilterValue, null>;
	created_from?: string;
	created_to?: string;
} => {
	const params: {
		query?: string;
		collection?: number;
		favorite?: number;
		submission?: number;
		watch_later?: number;
		status_filter?: Exclude<StatusFilterValue, null>;
		validation_filter?: Exclude<ValidationFilterValue, null>;
		created_from?: string;
		created_to?: string;
	} = {};

	if (state.query.trim()) {
		params.query = state.query;
	}

	if (state.videoSource && state.videoSource.type && state.videoSource.id) {
		const { type, id } = state.videoSource;
		params[type as 'collection' | 'favorite' | 'submission' | 'watch_later'] = parseInt(id);
	}
	if (state.statusFilter) {
		params.status_filter = state.statusFilter;
	}
	if (state.validationFilter) {
		params.validation_filter = state.validationFilter;
	}
	if (state.createdFrom) {
		params.created_from = state.createdFrom;
	}
	if (state.createdTo) {
		params.created_to = state.createdTo;
	}
	return params;
};

// 检查是否有活动的筛选条件
export const hasActiveFilters = (state: AppState): boolean => {
	return !!(
		state.query.trim() ||
		state.videoSource ||
		state.statusFilter ||
		state.validationFilter ||
		state.createdFrom ||
		state.createdTo
	);
};

export const setQuery = (query: string) => {
	appStateStore.update((state) => ({
		...state,
		query
	}));
};

export const setCurrentPage = (page: number) => {
	appStateStore.update((state) => ({
		...state,
		currentPage: page
	}));
};

export const setStatusFilter = (statusFilter: StatusFilterValue | null) => {
	appStateStore.update((state) => ({
		...state,
		statusFilter
	}));
};

export const setValidationFilter = (validationFilter: ValidationFilterValue | null) => {
	appStateStore.update((state) => ({
		...state,
		validationFilter
	}));
};

export const setCreatedTimeFilter = (createdFrom: string | null, createdTo: string | null) => {
	appStateStore.update((state) => ({
		...state,
		createdFrom,
		createdTo
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
	statusFilter: StatusFilterValue | null,
	validationFilter: ValidationFilterValue | null = 'normal',
	createdFrom: string | null = null,
	createdTo: string | null = null
) => {
	appStateStore.set({
		query,
		currentPage,
		videoSource,
		statusFilter,
		validationFilter,
		createdFrom,
		createdTo
	});
};
