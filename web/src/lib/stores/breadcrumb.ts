import { writable } from 'svelte/store';

export interface BreadcrumbItem {
	href?: string;
	label: string;
}

export const breadcrumbStore = writable<BreadcrumbItem[]>([]);

export function setBreadcrumb(items: BreadcrumbItem[]) {
	breadcrumbStore.set(items);
}
