import { writable } from 'svelte/store';

export interface BreadcrumbItem {
    href?: string;
    label: string;
    isActive?: boolean;
    onClick?: () => void;
}

export const breadcrumbStore = writable<BreadcrumbItem[]>([]);

export function setBreadcrumb(items: BreadcrumbItem[]) {
    breadcrumbStore.set(items);
}
