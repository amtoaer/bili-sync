import type {
	ApiResponse,
	VideoSourcesResponse,
	VideosRequest,
	VideosResponse,
	VideoResponse,
	ResetVideoResponse,
	ResetFilteredVideosResponse,
	UpdateVideoStatusRequest,
	UpdateVideoStatusResponse,
	ApiError,
	FavoritesResponse,
	CollectionsResponse,
	UppersResponse,
	InsertFavoriteRequest,
	InsertCollectionRequest,
	InsertSubmissionRequest,
	VideoSourcesDetailsResponse,
	UpdateVideoSourceRequest,
	Config,
	DashBoardResponse,
	SysInfo,
	TaskStatus,
	ResetVideoStatusRequest,
	UpdateVideoSourceResponse,
	Notifier,
	UpdateFilteredVideoStatusRequest,
	UpdateFilteredVideoStatusResponse,
	ResetFilteredVideoStatusRequest
} from './types';
import { wsManager } from './ws';

// API 基础配置
const API_BASE_URL = '/api';

// HTTP 客户端类
class ApiClient {
	private baseURL: string;
	private defaultHeaders: Record<string, string>;

	constructor(baseURL: string = API_BASE_URL) {
		this.baseURL = baseURL;
		this.defaultHeaders = {
			'Content-Type': 'application/json'
		};
		const token = localStorage.getItem('authToken');
		if (token) {
			this.defaultHeaders['Authorization'] = token;
		}
	}

	// 设置认证 token
	setAuthToken(token?: string) {
		if (token) {
			this.defaultHeaders['Authorization'] = token;
			localStorage.setItem('authToken', token);
		} else {
			delete this.defaultHeaders['Authorization'];
			localStorage.removeItem('authToken');
		}
	}

	// 清除认证 token
	clearAuthToken() {
		delete this.defaultHeaders['Authorization'];
		localStorage.removeItem('authToken');
		// 断开 WebSocket 连接，因为 token 已经无效
		wsManager.disconnect();
	}

	// 通用请求方法
	private async request<T>(
		url: string,
		method: string = 'GET',
		body?: unknown,
		params?: Record<string, unknown>
	): Promise<ApiResponse<T>> {
		// 构建完整的 URL
		let fullUrl = `${this.baseURL}${url}`;
		if (params) {
			const searchParams = new URLSearchParams();
			Object.entries(params).forEach(([key, value]) => {
				if (value !== undefined && value !== null) {
					searchParams.append(key, String(value));
				}
			});
			const queryString = searchParams.toString();
			if (queryString) {
				fullUrl += `?${queryString}`;
			}
		}

		const config: RequestInit = {
			method,
			headers: this.defaultHeaders
		};

		if (body && method !== 'GET') {
			config.body = JSON.stringify(body);
		}

		try {
			const response = await fetch(fullUrl, config);

			if (!response.ok) {
				const errorText = await response.text();
				let errorMessage: string;
				try {
					const errorJson = JSON.parse(errorText);
					errorMessage = errorJson.message || errorJson.error || '请求失败';
				} catch {
					errorMessage = errorText || `HTTP ${response.status}: ${response.statusText}`;
				}
				throw {
					message: errorMessage,
					status: response.status
				} as ApiError;
			}

			return await response.json();
		} catch (error) {
			if (error && typeof error === 'object' && 'status' in error) {
				throw error;
			}
			throw {
				message: error instanceof Error ? error.message : '网络请求失败',
				status: 0
			} as ApiError;
		}
	}

	// GET 请求
	private async get<T>(url: string, params?: Record<string, unknown>): Promise<ApiResponse<T>> {
		return this.request<T>(url, 'GET', undefined, params);
	}

	// POST 请求
	private async post<T>(url: string, data?: unknown): Promise<ApiResponse<T>> {
		return this.request<T>(url, 'POST', data);
	}

	// PUT 请求
	private async put<T>(url: string, data?: unknown): Promise<ApiResponse<T>> {
		return this.request<T>(url, 'PUT', data);
	}

	async getVideoSources(): Promise<ApiResponse<VideoSourcesResponse>> {
		return this.get<VideoSourcesResponse>('/video-sources');
	}

	async getVideos(params?: VideosRequest): Promise<ApiResponse<VideosResponse>> {
		return this.get<VideosResponse>('/videos', params as Record<string, unknown>);
	}

	async getVideo(id: number): Promise<ApiResponse<VideoResponse>> {
		return this.get<VideoResponse>(`/videos/${id}`);
	}

	async resetVideoStatus(
		id: number,
		request: ResetVideoStatusRequest
	): Promise<ApiResponse<ResetVideoResponse>> {
		return this.post<ResetVideoResponse>(`/videos/${id}/reset-status`, request);
	}

	async resetFilteredVideoStatus(
		request: ResetFilteredVideoStatusRequest
	): Promise<ApiResponse<ResetFilteredVideosResponse>> {
		return this.post<ResetFilteredVideosResponse>('/videos/reset-status', request);
	}

	async updateVideoStatus(
		id: number,
		request: UpdateVideoStatusRequest
	): Promise<ApiResponse<UpdateVideoStatusResponse>> {
		return this.post<UpdateVideoStatusResponse>(`/videos/${id}/update-status`, request);
	}

	async updateFilteredVideoStatus(
		request: UpdateFilteredVideoStatusRequest
	): Promise<ApiResponse<UpdateFilteredVideoStatusResponse>> {
		return this.post<UpdateFilteredVideoStatusResponse>('/videos/update-status', request);
	}

	async getCreatedFavorites(): Promise<ApiResponse<FavoritesResponse>> {
		return this.get<FavoritesResponse>('/me/favorites');
	}

	async getFollowedCollections(
		pageNum?: number,
		pageSize?: number
	): Promise<ApiResponse<CollectionsResponse>> {
		const params = {
			page_num: pageNum,
			page_size: pageSize
		};
		return this.get<CollectionsResponse>('/me/collections', params as Record<string, unknown>);
	}

	async getFollowedUppers(
		pageNum?: number,
		pageSize?: number
	): Promise<ApiResponse<UppersResponse>> {
		const params = {
			page_num: pageNum,
			page_size: pageSize
		};
		return this.get<UppersResponse>('/me/uppers', params as Record<string, unknown>);
	}

	async insertFavorite(request: InsertFavoriteRequest): Promise<ApiResponse<boolean>> {
		return this.post<boolean>('/video-sources/favorites', request);
	}

	async insertCollection(request: InsertCollectionRequest): Promise<ApiResponse<boolean>> {
		return this.post<boolean>('/video-sources/collections', request);
	}

	async insertSubmission(request: InsertSubmissionRequest): Promise<ApiResponse<boolean>> {
		return this.post<boolean>('/video-sources/submissions', request);
	}

	async getVideoSourcesDetails(): Promise<ApiResponse<VideoSourcesDetailsResponse>> {
		return this.get<VideoSourcesDetailsResponse>('/video-sources/details');
	}

	async updateVideoSource(
		type: string,
		id: number,
		request: UpdateVideoSourceRequest
	): Promise<ApiResponse<UpdateVideoSourceResponse>> {
		return this.put<UpdateVideoSourceResponse>(`/video-sources/${type}/${id}`, request);
	}

	async removeVideoSource(type: string, id: number): Promise<ApiResponse<boolean>> {
		return this.request<boolean>(`/video-sources/${type}/${id}`, 'DELETE');
	}

	async evaluateVideoSourceRules(type: string, id: number): Promise<ApiResponse<boolean>> {
		return this.post<boolean>(`/video-sources/${type}/${id}/evaluate`, null);
	}

	async getDefaultPath(type: string, name: string): Promise<ApiResponse<string>> {
		return this.get<string>(`/video-sources/${type}/default-path`, { name });
	}

	async testNotifier(notifier: Notifier): Promise<ApiResponse<boolean>> {
		return this.post<boolean>('/config/notifiers/ping', notifier);
	}

	async getConfig(): Promise<ApiResponse<Config>> {
		return this.get<Config>('/config');
	}

	async updateConfig(config: Config): Promise<ApiResponse<Config>> {
		return this.put<Config>('/config', config);
	}

	async getDashboard(): Promise<ApiResponse<DashBoardResponse>> {
		return this.get<DashBoardResponse>('/dashboard');
	}

	async triggerDownloadTask(): Promise<ApiResponse<boolean>> {
		return this.post<boolean>('/task/download');
	}

	subscribeToLogs(onMessage: (data: string) => void) {
		return wsManager.subscribeToLogs(onMessage);
	}
	subscribeToSysInfo(onMessage: (data: SysInfo) => void) {
		return wsManager.subscribeToSysInfo(onMessage);
	}
	subscribeToTasks(onMessage: (data: TaskStatus) => void) {
		return wsManager.subscribeToTasks(onMessage);
	}
}

// 创建默认的 API 客户端实例
export const apiClient = new ApiClient();

// 导出 API 方法的便捷函数
const api = {
	getVideoSources: () => apiClient.getVideoSources(),
	getVideos: (params?: VideosRequest) => apiClient.getVideos(params),
	getVideo: (id: number) => apiClient.getVideo(id),
	resetVideoStatus: (id: number, request: ResetVideoStatusRequest) =>
		apiClient.resetVideoStatus(id, request),
	resetFilteredVideoStatus: (request: ResetFilteredVideoStatusRequest) =>
		apiClient.resetFilteredVideoStatus(request),
	updateVideoStatus: (id: number, request: UpdateVideoStatusRequest) =>
		apiClient.updateVideoStatus(id, request),
	updateFilteredVideoStatus: (request: UpdateFilteredVideoStatusRequest) =>
		apiClient.updateFilteredVideoStatus(request),
	getCreatedFavorites: () => apiClient.getCreatedFavorites(),
	getFollowedCollections: (pageNum?: number, pageSize?: number) =>
		apiClient.getFollowedCollections(pageNum, pageSize),
	getFollowedUppers: (pageNum?: number, pageSize?: number) =>
		apiClient.getFollowedUppers(pageNum, pageSize),
	insertFavorite: (request: InsertFavoriteRequest) => apiClient.insertFavorite(request),
	insertCollection: (request: InsertCollectionRequest) => apiClient.insertCollection(request),
	insertSubmission: (request: InsertSubmissionRequest) => apiClient.insertSubmission(request),
	getVideoSourcesDetails: () => apiClient.getVideoSourcesDetails(),
	updateVideoSource: (type: string, id: number, request: UpdateVideoSourceRequest) =>
		apiClient.updateVideoSource(type, id, request),
	removeVideoSource: (type: string, id: number) => apiClient.removeVideoSource(type, id),
	evaluateVideoSourceRules: (type: string, id: number) =>
		apiClient.evaluateVideoSourceRules(type, id),
	getDefaultPath: (type: string, name: string) => apiClient.getDefaultPath(type, name),
	testNotifier: (notifier: Notifier) => apiClient.testNotifier(notifier),
	getConfig: () => apiClient.getConfig(),
	updateConfig: (config: Config) => apiClient.updateConfig(config),
	getDashboard: () => apiClient.getDashboard(),
	triggerDownloadTask: () => apiClient.triggerDownloadTask(),
	subscribeToSysInfo: (onMessage: (data: SysInfo) => void) =>
		apiClient.subscribeToSysInfo(onMessage),

	subscribeToLogs: (onMessage: (data: string) => void) => apiClient.subscribeToLogs(onMessage),

	subscribeToTasks: (onMessage: (data: TaskStatus) => void) =>
		apiClient.subscribeToTasks(onMessage),

	setAuthToken: (token: string) => apiClient.setAuthToken(token),
	clearAuthToken: () => apiClient.clearAuthToken()
};

export default api;
