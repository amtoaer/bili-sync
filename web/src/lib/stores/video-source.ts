import { writable } from 'svelte/store';
import { type VideoSourcesResponse } from '$lib/types';

export const videoSourceStore = writable<VideoSourcesResponse | undefined>(undefined);

// 便捷的设置和清除方法
export const setVideoSources = (sources: VideoSourcesResponse) => {
	videoSourceStore.set(sources);
};

export const clearFilter = () => {
	videoSourceStore.set(undefined);
};
