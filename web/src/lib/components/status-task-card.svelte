<!-- 可复用的状态任务卡片组件 -->
<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';

	export let taskName: string;
	export let currentStatus: number;
	export let originalStatus: number;
	export let onStatusChange: (newStatus: number) => void;
	export let onReset: () => void;
	export let disabled: boolean = false;

	// 获取状态显示信息
	function getStatusInfo(value: number) {
		if (value === 7) return { label: '已完成', class: 'text-green-600', dotClass: 'bg-green-500' };
		if (value >= 1 && value <= 4)
			return { label: `失败${value}次`, class: 'text-red-600', dotClass: 'bg-red-500' };
		return { label: '未开始', class: 'text-yellow-600', dotClass: 'bg-yellow-500' };
	}

	$: statusInfo = getStatusInfo(currentStatus);
	$: isModified = currentStatus !== originalStatus;
</script>

<div
	class="bg-background hover:bg-muted/30 flex items-center justify-between rounded-md border p-3 transition-colors {isModified
		? 'border-blue-200 ring-2 ring-blue-500/20'
		: ''}"
>
	<div class="flex items-center gap-3">
		<div>
			<div class="flex items-center gap-2">
				<span class="text-sm font-medium">{taskName}</span>
				{#if isModified}
					<span class="hidden text-xs font-medium text-blue-600 sm:inline">已修改</span>
					<div class="h-2 w-2 rounded-full bg-blue-500 sm:hidden" title="已修改"></div>
				{/if}
			</div>
			<div class="mt-0.5 flex items-center gap-1.5">
				<div class="h-1.5 w-1.5 rounded-full {statusInfo.dotClass}"></div>
				<span class="text-xs {statusInfo.class}">{statusInfo.label}</span>
			</div>
		</div>
	</div>
	<div class="flex gap-1.5">
		{#if isModified}
			<Button
				variant="ghost"
				size="sm"
				onclick={onReset}
				{disabled}
				class="h-7 min-w-[60px] cursor-pointer px-3 text-xs text-gray-600 hover:bg-gray-100"
				title="恢复到原始状态"
			>
				重置
			</Button>
		{/if}
		<Button
			variant={currentStatus === 0 ? 'default' : 'outline'}
			size="sm"
			onclick={() => onStatusChange(0)}
			{disabled}
			class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {currentStatus === 0
				? 'border-yellow-600 bg-yellow-600 font-medium text-white hover:bg-yellow-700'
				: 'hover:border-yellow-400 hover:bg-yellow-50 hover:text-yellow-700'}"
		>
			未开始
		</Button>
		<Button
			variant={currentStatus === 7 ? 'default' : 'outline'}
			size="sm"
			onclick={() => onStatusChange(7)}
			{disabled}
			class="h-7 min-w-[60px] cursor-pointer px-3 text-xs {currentStatus === 7
				? 'border-green-600 bg-green-600 font-medium text-white hover:bg-green-700'
				: 'hover:border-green-400 hover:bg-green-50 hover:text-green-700'}"
		>
			已完成
		</Button>
	</div>
</div>
