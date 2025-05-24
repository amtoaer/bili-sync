import type { VideoResponse, VideoInfo, VideosResponse, VideoSourcesResponse, ResetVideoResponse, AddVideoSourceResponse, DeleteVideoSourceResponse } from './types';

const BASE_URL = '/api';

export class ApiError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ApiError';
    }
}

async function fetchWithAuth(url: string, options: RequestInit = {}) {
    try {
    const token = localStorage.getItem('auth_token');
    const headers = {
        ...options.headers,
        'Authorization': token || ''
    };

        console.log(`请求: ${url}`, options.method || 'GET');

    const response = await fetch(url, { ...options, headers });
    if (!response.ok) {
            const errorText = await response.text();
            console.error(`API请求失败: ${response.status} ${response.statusText}`, errorText);
            throw new ApiError(`API请求失败: ${response.status} ${response.statusText}, 响应: ${errorText}`);
    }
        
        const responseData = await response.json();
        if (!responseData.data && url !== `${BASE_URL}/video-sources`) {
            console.warn(`API响应缺少data字段:`, responseData);
        }
        
        return responseData.data;
    } catch (error) {
        console.error(`请求 ${url} 时出错:`, error);
        throw error;
    }
}

export async function getVideoSources(): Promise<VideoSourcesResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources`);
}

export async function listVideos(params: {
    collection?: string;
    favorite?: string;
    submission?: string;
    watch_later?: string;
    bangumi?: string;
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

// 添加新的视频源
export async function addVideoSource(params: {
    source_type: string;
    source_id: string;
    name: string;
    path: string;
    collection_type?: string;
    media_id?: string;
    ep_id?: string;
    download_all_seasons?: boolean;
}): Promise<AddVideoSourceResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(params)
    });
}

// 删除视频源
export async function deleteVideoSource(source_type: string, id: number, delete_local_files: boolean = false): Promise<DeleteVideoSourceResponse> {
    return fetchWithAuth(`${BASE_URL}/video-sources/${source_type}/${id}?delete_local_files=${delete_local_files}`, {
        method: 'DELETE'
    });
}