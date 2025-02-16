import type { VideoCount, VideoDetail, VideoInfo, VideoListModel } from './types';

const BASE_URL = '/api';

export class ApiError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

async function fetchWithAuth(url: string, options: RequestInit = {}) {
    const token = localStorage.getItem('auth_token');
    const headers = {
        ...options.headers,
        'Authorization': token || ''
    };

    const response = await fetch(url, { ...options, headers });
    if (!response.ok) {
        throw new ApiError(`API request failed: ${response.statusText}`);
    }
    return response.json();
}

export async function getVideoListModels(): Promise<VideoListModel> {
    return fetchWithAuth(`${BASE_URL}/video-list-models`);
}

export async function listVideos(params: {
    collection?: string;
    favorite?: string;
    submission?: string;
    watch_later?: string;
    count?: number;
    q?: string;
    o?: number;
    l?: number;
}): Promise<VideoInfo[] | VideoCount> {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
            searchParams.append(key, value.toString());
        }
    });
    return fetchWithAuth(`${BASE_URL}/videos?${searchParams.toString()}`);
}


export async function getVideo(id: number): Promise<VideoDetail> {
    return fetchWithAuth(`${BASE_URL}/videos/${id}`);
}
