import { writable } from 'svelte/store';

export interface AppState {
    query: string;
    videoSource: {
        key: string;
        value: string;
    };
}

// 创建应用状态store
export const appStateStore = writable<AppState>({
    query: '',
    videoSource: {
        key: '',
        value: ''
    }
});

// 便捷的设置方法
export const setQuery = (query: string) => {
    appStateStore.update(state => ({
        ...state,
        query
    }));
};

export const setVideoSourceFilter = (key: string, value: string) => {
    appStateStore.update(state => ({
        ...state,
        videoSource: { key, value }
    }));
};

export const clearVideoSourceFilter = () => {
    appStateStore.update(state => ({
        ...state,
        videoSource: { key: '', value: '' }
    }));
};

export const clearAll = () => {
    appStateStore.set({
        query: '',
        videoSource: { key: '', value: '' }
    });
};

// 保留旧的接口以兼容现有代码
export const filterStore = writable({ key: '', value: '' });
export const setFilter = setVideoSourceFilter;
export const clearFilter = clearVideoSourceFilter;
