<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type Props = WithElementRef<
		HTMLInputAttributes & {
			class?: string;
		}
	>;

	let {
		ref = $bindable(null),
		value = $bindable(''),
		class: className,
		placeholder = '',
		...restProps
	}: Props = $props();

	// 密码可见性状态
	let visible = $state(false);

	// 切换密码可见性
	function toggleVisibility() {
		visible = !visible;
	}
</script>

<div class="relative">
	<input
		bind:this={ref}
		data-slot="input"
		class={cn(
			'border-input bg-background selection:bg-primary dark:bg-input/30 selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex h-9 w-full min-w-0 rounded-md border px-3 py-1 pr-10 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
			'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]',
			'aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive',
			className
		)}
		type={visible ? 'text' : 'password'}
		{placeholder}
		bind:value
		{...restProps}
	/>
	<button
		type="button"
		class="text-muted-foreground hover:text-foreground absolute top-1/2 right-3 -translate-y-1/2"
		onclick={toggleVisibility}
		aria-label={visible ? '隐藏密码' : '显示密码'}
		tabindex="-1"
	>
		{#if visible}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"></path>
				<path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68"
				></path>
				<path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61"></path>
				<line x1="2" x2="22" y1="2" y2="22"></line>
			</svg>
		{:else}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"></path>
				<circle cx="12" cy="12" r="3"></circle>
			</svg>
		{/if}
	</button>
</div>
