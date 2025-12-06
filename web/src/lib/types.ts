export interface ApiResponse<T> {
	status_code: number;
	data: T;
}

export interface VideosRequest {
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	query?: string;
	page?: number;
	page_size?: number;
}

export interface VideoSource {
	id: number;
	name: string;
}

export interface VideoSourcesResponse {
	collection: VideoSource[];
	favorite: VideoSource[];
	submission: VideoSource[];
	watch_later: VideoSource[];
}

export interface VideoInfo {
	id: number;
	bvid: string;
	name: string;
	upper_name: string;
	should_download: boolean;
	download_status: [number, number, number, number, number];
}

export interface VideosResponse {
	videos: VideoInfo[];
	total_count: number;
}

export interface PageInfo {
	id: number;
	pid: number;
	name: string;
	download_status: [number, number, number, number, number];
}

export interface VideoResponse {
	video: VideoInfo;
	pages: PageInfo[];
}

export interface ResetVideoResponse {
	resetted: boolean;
	video: VideoInfo;
	pages: PageInfo[];
}

export interface ResetFilteredVideosResponse {
	resetted: boolean;
	resetted_videos_count: number;
	resetted_pages_count: number;
}

export interface UpdateVideoStatusResponse {
	success: boolean;
	video: VideoInfo;
	pages: PageInfo[];
}

export interface UpdateFilteredVideoStatusResponse {
	success: boolean;
	updated_videos_count: number;
	updated_pages_count: number;
}

export interface ApiError {
	message: string;
	status?: number;
}

export interface StatusUpdate {
	status_index: number;
	status_value: number;
}

export interface PageStatusUpdate {
	page_id: number;
	updates: StatusUpdate[];
}

export interface UpdateVideoStatusRequest {
	video_updates?: StatusUpdate[];
	page_updates?: PageStatusUpdate[];
}

export interface UpdateFilteredVideoStatusRequest {
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	query?: string;
	video_updates?: StatusUpdate[];
	page_updates?: StatusUpdate[];
}

export interface ResetVideoStatusRequest {
	force: boolean;
}

export interface ResetFilteredVideoStatusRequest {
	collection?: number;
	favorite?: number;
	submission?: number;
	watch_later?: number;
	query?: string;
	force: boolean;
}

export type Followed =
	| {
			type: 'favorite';
			title: string;
			media_count: number;
			fid: number;
			mid: number;
			invalid: boolean;
			subscribed: boolean;
	  }
	| {
			type: 'collection';
			title: string;
			sid: number;
			mid: number;
			media_count: number;
			invalid: boolean;
			subscribed: boolean;
	  }
	| {
			type: 'upper';
			mid: number;
			uname: string;
			face: string;
			sign: string;
			invalid: boolean;
			subscribed: boolean;
	  };

export interface FavoritesResponse {
	favorites: Followed[];
}

export interface CollectionsResponse {
	collections: Followed[];
	total: number;
}

export interface UppersResponse {
	uppers: Followed[];
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

export interface VideoSourceDetail {
	id: number;
	name: string;
	path: string;
	rule: Rule | null;
	ruleDisplay: string | null;
	useDynamicApi: boolean | null;
	enabled: boolean;
}

export interface VideoSourcesDetailsResponse {
	collections: VideoSourceDetail[];
	favorites: VideoSourceDetail[];
	submissions: VideoSourceDetail[];
	watch_later: VideoSourceDetail[];
}

export interface UpdateVideoSourceRequest {
	path: string;
	enabled: boolean;
	rule?: Rule | null;
	useDynamicApi?: boolean | null;
}

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

export interface TelegramNotifier {
	type: 'telegram';
	bot_token: string;
	chat_id: string;
}

export interface WebhookNotifier {
	type: 'webhook';
	url: string;
	template?: string | null;
}

export type Notifier = TelegramNotifier | WebhookNotifier;

export type Trigger = number | string;

export interface Config {
	auth_token: string;
	bind_address: string;
	credential: Credential;
	filter_option: FilterOption;
	danmaku_option: DanmakuOption;
	skip_option: SkipOption;
	video_name: string;
	page_name: string;
	notifiers: Notifier[] | null;
	favorite_default_path: string;
	collection_default_path: string;
	submission_default_path: string;
	interval: Trigger;
	upper_path: string;
	nfo_time_type: string;
	concurrent_limit: ConcurrentLimit;
	time_format: string;
	cdn_sorting: boolean;
	version: number;
}

export interface DayCountPair {
	day: string;
	cnt: number;
}

export interface DashBoardResponse {
	enabled_favorites: number;
	enabled_collections: number;
	enabled_submissions: number;
	enable_watch_later: boolean;
	videos_by_day: DayCountPair[];
}

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
