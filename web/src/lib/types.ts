export interface VideoSource {
    id: number;
    name: string;
}

export interface VideoSourcesResponse {
    collection: VideoSource[];
    favorite: VideoSource[];
    submission: VideoSource[];
    watch_later: VideoSource[];
    bangumi: VideoSource[];
}

export interface VideoInfo {
    id: number;
    name: string;
    upper_name: string;
    download_status: number[];
}

export interface VideosResponse {
    videos: VideoInfo[];
    total_count: number;
}

export interface PageInfo {
    id: number;
    pid: number;
    name: string;
    download_status: number[];
}

export interface VideoResponse {
    video: VideoInfo;
    pages: PageInfo[];
}

export interface ResetVideoResponse {
    resetted: boolean;
    video: number;
    pages: number[];
}

export interface AddVideoSourceResponse {
    success: boolean;
    source_id: number;
    source_type: string;
    message: string;
}

export interface DeleteVideoSourceResponse {
    success: boolean;
    source_id: number;
    source_type: string;
    message: string;
}

export type VideoCategory = 'collection' | 'favorite' | 'submission' | 'watch_later' | 'bangumi';
