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
	bvid: string;
	name: string;
	upper_name: string;
	should_download: boolean;
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
	video: VideoInfo;
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

// 状态更新类型
export interface StatusUpdate {
	status_index: number;
	status_value: number;
}

// 页面状态更新类型
export interface PageStatusUpdate {
	page_id: number;
	updates: StatusUpdate[];
}

// 重置视频状态请求类型
export interface UpdateVideoStatusRequest {
	video_updates?: StatusUpdate[];
	page_updates?: PageStatusUpdate[];
}

// 重置视频状态响应类型
export interface UpdateVideoStatusResponse {
	success: boolean;
	video: VideoInfo;
	pages: PageInfo[];
}

// 重置请求类型
export interface ResetRequest {
	force: boolean;
}

// 收藏夹相关类型
export interface FavoriteWithSubscriptionStatus {
	title: string;
	media_count: number;
	fid: number;
	mid: number;
	subscribed: boolean;
}

export interface FavoritesResponse {
	favorites: FavoriteWithSubscriptionStatus[];
}

// 合集相关类型
export interface CollectionWithSubscriptionStatus {
	title: string;
	sid: number;
	mid: number;
	invalid: boolean;
	subscribed: boolean;
}

export interface CollectionsResponse {
	collections: CollectionWithSubscriptionStatus[];
	total: number;
}

// UP 主相关类型
export interface UpperWithSubscriptionStatus {
	mid: number;
	uname: string;
	face: string;
	sign: string;
	invalid: boolean;
	subscribed: boolean;
}

export interface UppersResponse {
	uppers: UpperWithSubscriptionStatus[];
	total: number;
}

export interface InsertFavoriteRequest {
	fid: number;
	path: string;
}

export interface InsertCollectionRequest {
	sid: number;
	mid: number;
	collection_type?: number;
	path: string;
}

export interface InsertSubmissionRequest {
	upper_id: number;
	path: string;
}

// Rule 相关类型
export interface Condition<T> {
	operator: string;
	value: T | T[];
}

export interface RuleTarget<T> {
	field: string;
	rule: Condition<T> | RuleTarget<T>;
}

export type AndGroup = RuleTarget<string | number | Date>[];
export type Rule = AndGroup[];

// 视频源详细信息类型
export interface VideoSourceDetail {
	id: number;
	name: string;
	path: string;
	rule: Rule | null;
	ruleDisplay: string | null;
	useDynamicApi: boolean | null;
	enabled: boolean;
}

// 视频源详细信息响应类型
export interface VideoSourcesDetailsResponse {
	collections: VideoSourceDetail[];
	favorites: VideoSourceDetail[];
	submissions: VideoSourceDetail[];
	watch_later: VideoSourceDetail[];
}

// 更新视频源请求类型
export interface UpdateVideoSourceRequest {
	path: string;
	enabled: boolean;
	rule?: Rule | null;
	useDynamicApi?: boolean | null;
}

// 配置相关类型
export interface Credential {
	sessdata: string;
	bili_jct: string;
	buvid3: string;
	dedeuserid: string;
	ac_time_value: string;
}

export interface FilterOption {
	video_max_quality: string;
	video_min_quality: string;
	audio_max_quality: string;
	audio_min_quality: string;
	codecs: string[];
	no_dolby_video: boolean;
	no_dolby_audio: boolean;
	no_hdr: boolean;
	no_hires: boolean;
}

export interface DanmakuOption {
	duration: number;
	font: string;
	font_size: number;
	width_ratio: number;
	horizontal_gap: number;
	lane_size: number;
	float_percentage: number;
	bottom_percentage: number;
	opacity: number;
	bold: boolean;
	outline: number;
	time_offset: number;
}

export interface SkipOption {
	no_poster: boolean;
	no_video_nfo: boolean;
	no_upper: boolean;
	no_danmaku: boolean;
	no_subtitle: boolean;
}

export interface RateLimit {
	limit: number;
	duration: number;
}

export interface ConcurrentDownloadLimit {
	enable: boolean;
	concurrency: number;
	threshold: number;
}

export interface ConcurrentLimit {
	video: number;
	page: number;
	rate_limit?: RateLimit;
	download: ConcurrentDownloadLimit;
}

export interface Config {
	auth_token: string;
	bind_address: string;
	credential: Credential;
	filter_option: FilterOption;
	danmaku_option: DanmakuOption;
	skip_option: SkipOption;
	video_name: string;
	page_name: string;
	interval: number;
	upper_path: string;
	nfo_time_type: string;
	concurrent_limit: ConcurrentLimit;
	time_format: string;
	cdn_sorting: boolean;
	version: number;
}

// 日期计数对类型
export interface DayCountPair {
	day: string;
	cnt: number;
}

// 仪表盘响应类型
export interface DashBoardResponse {
	enabled_favorites: number;
	enabled_collections: number;
	enabled_submissions: number;
	enable_watch_later: boolean;
	videos_by_day: DayCountPair[];
}

// 系统信息响应类型
export interface SysInfo {
	total_memory: number;
	used_memory: number;
	process_memory: number;
	used_cpu: number;
	process_cpu: number;
	total_disk: number;
	used_disk: number;
	available_disk: number;
	uptime: number;
}

export interface TaskStatus {
	is_running: boolean;
	last_run: Date | null;
	last_finish: Date | null;
	next_run: Date | null;
}

export interface UpdateVideoSourceResponse {
	ruleDisplay: string;
}
