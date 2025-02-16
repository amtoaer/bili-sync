export interface VideoListModelItem {
    id: number;
    name: string;
}

export interface VideoListModel {
    collection: VideoListModelItem[];
    favorite: VideoListModelItem[];
    submission: VideoListModelItem[];
    watch_later: VideoListModelItem[];
}

export interface VideoInfo {
    id: number;
    name: string;
    upper_name: string;
    download_status: number[];
}

export interface VideoCount {
    count: number;
}

export interface PageInfo {
    id: number;
    pid: number;
    name: string;
    download_status: number[];
}

export interface VideoDetail {
    video: VideoInfo;
    pages: PageInfo[];
}

export type VideoCategory = 'collection' | 'favorite' | 'submission' | 'watch_later';
