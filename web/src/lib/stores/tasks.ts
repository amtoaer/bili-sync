import { writable } from 'svelte/store';


export interface TaskStatus {
	is_running: boolean;
    last_run: Date | null;
    last_finish: Date | null;
    next_run: Date | null;
}

export const taskStatusStore = writable<TaskStatus>(undefined);

export function setTaskStatus(status: TaskStatus) {
    taskStatusStore.set(status);
}
