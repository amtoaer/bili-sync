<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import {
		ChevronLeftIcon,
		ChevronRightIcon,
		ChevronsLeftIcon,
		ChevronsRightIcon
	} from '@lucide/svelte/icons';

	export let currentPage: number = 0;
	export let totalPages: number = 0;
	export let onPageChange: (page: number) => void = () => {};

	function goToPage(page: number) {
		if (page >= 0 && page < totalPages && page !== currentPage) {
			onPageChange(page);
		}
	}

	function getVisiblePages(current: number, total: number): number[] {
		const pages: number[] = [];
		const maxVisible = 7; // 显示最多7个页码按钮

		if (total <= maxVisible) {
			// 如果总页数少于最大显示数，显示所有页
			for (let i = 0; i < total; i++) {
				pages.push(i);
			}
		} else {
			// 计算显示范围
			let start = Math.max(0, current - 3);
			let end = Math.min(total - 1, start + maxVisible - 1);

			// 如果end达到边界，调整start
			if (end === total - 1) {
				start = Math.max(0, end - maxVisible + 1);
			}

			for (let i = start; i <= end; i++) {
				pages.push(i);
			}
		}

		return pages;
	}

	$: visiblePages = getVisiblePages(currentPage, totalPages);
	$: hasPrevious = currentPage > 0;
	$: hasNext = currentPage < totalPages - 1;
</script>

{#if totalPages > 1}
	<div class="mt-8 flex items-center justify-center gap-1">
		<!-- 第一页 -->
		<Button
			variant="outline"
			size="sm"
			class="h-8 w-8 cursor-pointer p-0"
			disabled={!hasPrevious}
			onclick={() => goToPage(0)}
		>
			<ChevronsLeftIcon class="h-4 w-4" />
		</Button>

		<!-- 上一页 -->
		<Button
			variant="outline"
			size="sm"
			class="h-8 w-8 cursor-pointer p-0"
			disabled={!hasPrevious}
			onclick={() => goToPage(currentPage - 1)}
		>
			<ChevronLeftIcon class="h-4 w-4" />
		</Button>

		<!-- 页码按钮 -->
		{#each visiblePages as page (page)}
			<Button
				variant={page === currentPage ? 'default' : 'outline'}
				size="sm"
				class="h-8 min-w-8 cursor-pointer"
				onclick={() => goToPage(page)}
			>
				{page + 1}
			</Button>
		{/each}

		<!-- 下一页 -->
		<Button
			variant="outline"
			size="sm"
			class="h-8 w-8 cursor-pointer p-0"
			disabled={!hasNext}
			onclick={() => goToPage(currentPage + 1)}
		>
			<ChevronRightIcon class="h-4 w-4" />
		</Button>

		<!-- 最后一页 -->
		<Button
			variant="outline"
			size="sm"
			class="h-8 w-8 cursor-pointer p-0"
			disabled={!hasNext}
			onclick={() => goToPage(totalPages - 1)}
		>
			<ChevronsRightIcon class="h-4 w-4" />
		</Button>
	</div>
{/if}
