import type {
	ApiResponse,
	VideoSourcesResponse,
	VideosRequest,
	VideosResponse,
	VideoResponse,
	ResetVideoResponse,
	ResetAllVideosResponse,
	ResetVideoStatusRequest,
	ResetVideoStatusResponse,
	ApiError
} from './types';

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

	// 通用请求方法
	private async request<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResponse<T>> {
		const url = `${this.baseURL}${endpoint}`;

		const config: RequestInit = {
			headers: {
				...this.defaultHeaders,
				...options.headers
			},
			...options
		};

		try {
			const response = await fetch(url, config);

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data: ApiResponse<T> = await response.json();
			return data;
		} catch (error) {
			const apiError: ApiError = {
				message: error instanceof Error ? error.message : 'Unknown error occurred',
				status: error instanceof TypeError ? undefined : (error as { status?: number }).status
			};
			throw apiError;
		}
	}

	// GET 请求
	private async get<T>(
		endpoint: string,
		params?: VideosRequest | Record<string, unknown>
	): Promise<ApiResponse<T>> {
		let queryString = '';

		if (params) {
			const searchParams = new URLSearchParams();
			Object.entries(params).forEach(([key, value]) => {
				if (value !== undefined && value !== null) {
					searchParams.append(key, String(value));
				}
			});
			queryString = searchParams.toString();
		}

		const finalEndpoint = queryString ? `${endpoint}?${queryString}` : endpoint;
		return this.request<T>(finalEndpoint, {
			method: 'GET'
		});
	}

	// POST 请求
	private async post<T>(endpoint: string, data?: unknown): Promise<ApiResponse<T>> {
		return this.request<T>(endpoint, {
			method: 'POST',
			body: data ? JSON.stringify(data) : undefined
		});
	}

	// PUT 请求
	private async put<T>(endpoint: string, data?: unknown): Promise<ApiResponse<T>> {
		return this.request<T>(endpoint, {
			method: 'PUT',
			body: data ? JSON.stringify(data) : undefined
		});
	}

	// DELETE 请求
	private async delete<T>(endpoint: string): Promise<ApiResponse<T>> {
		return this.request<T>(endpoint, {
			method: 'DELETE'
		});
	}

	// API 方法

	/**
	 * 获取所有视频来源
	 */
	async getVideoSources(): Promise<ApiResponse<VideoSourcesResponse>> {
		return this.get<VideoSourcesResponse>('/video-sources');
	}

	/**
	 * 获取视频列表
	 * @param params 查询参数
	 */
	async getVideos(params?: VideosRequest): Promise<ApiResponse<VideosResponse>> {
		return this.get<VideosResponse>('/videos', params);
	}

	/**
	 * 获取单个视频详情
	 * @param id 视频 ID
	 */
	async getVideo(id: number): Promise<ApiResponse<VideoResponse>> {
		return this.get<VideoResponse>(`/videos/${id}`);
	}

	/**
	 * 重置视频下载状态
	 * @param id 视频 ID
	 */
	async resetVideo(id: number): Promise<ApiResponse<ResetVideoResponse>> {
		return this.post<ResetVideoResponse>(`/videos/${id}/reset`);
	}

	/**
	 * 重置所有视频下载状态
	 */
	async resetAllVideos(): Promise<ApiResponse<ResetAllVideosResponse>> {
		return this.post<ResetAllVideosResponse>('/videos/reset-all');
	}

	/**
	 * 重置视频状态位
	 * @param id 视频 ID
	 * @param request 重置请求参数
	 */
	async resetVideoStatus(id: number, request: ResetVideoStatusRequest): Promise<ApiResponse<ResetVideoStatusResponse>> {
		return this.post<ResetVideoStatusResponse>(`/videos/${id}/reset-status`, request);
	}
}

// 创建默认的 API 客户端实例
export const apiClient = new ApiClient();

// 导出 API 方法的便捷函数
export const api = {
	/**
	 * 获取所有视频来源
	 */
	getVideoSources: () => apiClient.getVideoSources(),

	/**
	 * 获取视频列表
	 */
	getVideos: (params?: VideosRequest) => apiClient.getVideos(params),

	/**
	 * 获取单个视频详情
	 */
	getVideo: (id: number) => apiClient.getVideo(id),

	/**
	 * 重置视频下载状态
	 */
	resetVideo: (id: number) => apiClient.resetVideo(id),

	/**
	 * 重置所有视频下载状态
	 */
	resetAllVideos: () => apiClient.resetAllVideos(),

	/**
	 * 重置视频状态位
	 */
	resetVideoStatus: (id: number, request: ResetVideoStatusRequest) => apiClient.resetVideoStatus(id, request),

	/**
	 * 设置认证 token
	 */
	setAuthToken: (token: string) => apiClient.setAuthToken(token)
};

// 默认导出
export default api;
