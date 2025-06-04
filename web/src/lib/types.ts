// API 响应包装器

export interface ApiResponse<T> {
	status_code: number;
	data: T;
}

// 请求参数类型
export interface VideosRequest {
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	query?: string;
	page?: number;
	page_size?: number;
}

// 视频来源类型
export interface VideoSource {
	id: number;
	name: string;
}

// 视频来源响应类型
export interface VideoSourcesResponse {
	collection: VideoSource[];
	favorite: VideoSource[];
	submission: VideoSource[];
	watch_later: VideoSource[];
}

// 视频信息类型
export interface VideoInfo {
	id: number;
	name: string;
	upper_name: string;
	download_status: [number, number, number, number, number];
}

// 视频列表响应类型
export interface VideosResponse {
	videos: VideoInfo[];
	total_count: number;
}

// 分页信息类型
export interface PageInfo {
	id: number;
	pid: number;
	name: string;
	download_status: [number, number, number, number, number];
}

// 单个视频响应类型
export interface VideoResponse {
	video: VideoInfo;
	pages: PageInfo[];
}

// 重置视频响应类型
export interface ResetVideoResponse {
	resetted: boolean;
	video: VideoInfo,
	pages: PageInfo[];
}

// 重置所有视频响应类型
export interface ResetAllVideosResponse {
	resetted: boolean;
	resetted_videos_count: number;
	resetted_pages_count: number;
}

// API 错误类型
export interface ApiError {
	message: string;
	status?: number;
}
