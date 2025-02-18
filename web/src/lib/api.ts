import type { VideoResponse, VideoInfo, VideosResponse, VideoSourcesResponse, ResetVideoResponse } from './types';

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
        throw new ApiError(`API request failed: ${response.statusText}, body: ${await response.text()}`);
    }
    let { data } = await response.json();
    return data;
}

export async function getVideoSources(): Promise<VideoSourcesResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources`);
}

export async function listVideos(params: {
    collection?: string;
    favorite?: string;
    submission?: string;
    watch_later?: string;
    query?: string;
    page?: number;
    page_size?: number;
}): Promise<VideosResponse> {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
            searchParams.append(key, value.toString());
        }
    });
    return fetchWithAuth(`${BASE_URL}/videos?${searchParams.toString()}`);
}


export async function getVideo(id: number): Promise<VideoResponse> {
    return fetchWithAuth(`${BASE_URL}/videos/${id}`);
}

export async function resetVideo(id: number): Promise<ResetVideoResponse> {
    return fetchWithAuth(`${BASE_URL}/videos/${id}/reset`, { method: 'POST' });
}