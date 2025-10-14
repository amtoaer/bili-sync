<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import SubscriptionDialog from './subscription-dialog.svelte';
	import UserIcon from '@lucide/svelte/icons/user';
	import VideoIcon from '@lucide/svelte/icons/video';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import CheckIcon from '@lucide/svelte/icons/check';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import type {
		FavoriteWithSubscriptionStatus,
		CollectionWithSubscriptionStatus,
		UpperWithSubscriptionStatus
	} from '$lib/types';

	export let item:
		| FavoriteWithSubscriptionStatus
		| CollectionWithSubscriptionStatus
		| UpperWithSubscriptionStatus;
	export let type: 'favorites' | 'collections' | 'submissions' = 'favorites';
	export let onSubscriptionSuccess: (() => void) | null = null;

	let dialogOpen = false;

	function getIcon() {
		switch (type) {
			case 'favorites':
				return HeartIcon;
			case 'collections':
				return FolderIcon;
			case 'submissions':
				return UserIcon;
			default:
				return VideoIcon;
		}
	}

	function getTypeLabel() {
		switch (type) {
			case 'favorites':
				return '收藏夹';
			case 'collections':
				return '合集';
			case 'submissions':
				return 'UP 主';
			default:
				return '';
		}
	}

	function getTitle(): string {
		switch (type) {
			case 'favorites':
				return (item as FavoriteWithSubscriptionStatus).title;
			case 'collections':
				return (item as CollectionWithSubscriptionStatus).title;
			case 'submissions':
				return (item as UpperWithSubscriptionStatus).uname;
			default:
				return '';
		}
	}

	function getSubtitle(): string {
		switch (type) {
			case 'favorites':
				return `uid: ${(item as FavoriteWithSubscriptionStatus).mid}`;
			case 'collections':
				return `uid: ${(item as CollectionWithSubscriptionStatus).mid}`;
			default:
				return '';
		}
	}

	function getDescription(): string {
		switch (type) {
			case 'submissions':
				return (item as UpperWithSubscriptionStatus).sign || '';
			default:
				return '';
		}
	}

	function isDisabled(): boolean {
		switch (type) {
			case 'collections':
				return (item as CollectionWithSubscriptionStatus).invalid;
			case 'submissions': {
				return (item as UpperWithSubscriptionStatus).invalid;
			}
			default:
				return false;
		}
	}

	function getDisabledReason(): string {
		switch (type) {
			case 'collections':
				return '已失效';
			case 'submissions':
				return '账号已注销';
			default:
				return '';
		}
	}

	function getCount(): number | null {
		switch (type) {
			case 'favorites':
				return (item as FavoriteWithSubscriptionStatus).media_count;
			default:
				return null;
		}
	}

	function getCountLabel(): string {
		return '个视频';
	}

	function getAvatarUrl(): string {
		switch (type) {
			case 'submissions':
				return (item as UpperWithSubscriptionStatus).face;
			default:
				return '';
		}
	}

	function handleSubscribe() {
		if (!disabled) {
			dialogOpen = true;
		}
	}

	function handleSubscriptionSuccess() {
		// 更新本地状态
		item.subscribed = true;
		if (onSubscriptionSuccess) {
			onSubscriptionSuccess();
		}
	}

	const Icon = getIcon();
	const typeLabel = getTypeLabel();
	const title = getTitle();
	const subtitle = getSubtitle();
	const description = getDescription();
	const count = getCount();
	const countLabel = getCountLabel();
	const avatarUrl = getAvatarUrl();
	const subscribed = item.subscribed;
	const disabled = isDisabled();
	const disabledReason = getDisabledReason();
</script>

<Card
	class="hover:shadow-primary/5 border-border/50 group flex h-full flex-col transition-all hover:shadow-lg {disabled
		? 'opacity-60'
		: ''}"
>
	<CardHeader class="flex-shrink-0 pb-4">
		<div class="flex items-start gap-3">
			<!-- 头像或图标 - 简化设计 -->
			<div
				class="bg-accent/50 flex h-10 w-10 shrink-0 items-center justify-center rounded-full {disabled
					? 'opacity-50'
					: ''}"
			>
				{#if avatarUrl && type === 'submissions'}
					<img
						src={avatarUrl}
						alt={title}
						class="h-full w-full rounded-full object-cover {disabled ? 'grayscale' : ''}"
						loading="lazy"
					/>
				{:else}
					<Icon class="text-muted-foreground h-5 w-5" />
				{/if}
			</div>

			<!-- 内容区域 -->
			<div class="min-w-0 flex-1 space-y-2">
				<div class="flex items-start justify-between gap-2">
					<CardTitle
						class="line-clamp-2 text-sm leading-relaxed font-medium {disabled
							? 'text-muted-foreground line-through'
							: ''}"
						{title}
					>
						{title}
					</CardTitle>

					<!-- 状态标记 -->
					{#if disabled}
						<Badge variant="destructive" class="shrink-0 text-xs">不可用</Badge>
					{:else}
						<Badge variant={subscribed ? 'outline' : 'secondary'} class="shrink-0 text-xs">
							{subscribed ? '已订阅' : typeLabel}
						</Badge>
					{/if}
				</div>

				<!-- 副标题和描述 -->
				{#if subtitle && !disabled}
					<div class="text-muted-foreground flex items-center gap-1 text-sm">
						<UserIcon class="h-3 w-3 shrink-0" />
						<span class="truncate" title={subtitle}>{subtitle}</span>
					</div>
				{/if}

				{#if description && !disabled}
					<p class="text-muted-foreground line-clamp-1 text-sm" title={description}>
						{description}
					</p>
				{/if}

				<!-- 计数信息 -->
				{#if count !== null && !disabled}
					<div class="text-muted-foreground text-sm">
						{count}
						{countLabel}
					</div>
				{/if}
			</div>
		</div>
	</CardHeader>

	<!-- 底部按钮区域 -->
	<CardContent class="flex min-w-0 flex-1 flex-col justify-end pt-0 pb-4">
		<div class="flex justify-end">
			{#if disabled}
				<Button
					size="sm"
					variant="outline"
					disabled
					class="h-8 cursor-not-allowed text-xs opacity-50"
				>
					<XIcon class="mr-1 h-3 w-3" />
					{disabledReason}
				</Button>
			{:else if subscribed}
				<Button size="sm" variant="outline" disabled class="h-8 cursor-not-allowed text-xs">
					<CheckIcon class="mr-1 h-3 w-3" />
					已订阅
				</Button>
			{:else}
				<Button
					size="sm"
					variant="outline"
					onclick={handleSubscribe}
					class="h-8 cursor-pointer text-xs font-medium"
				>
					<PlusIcon class="mr-1 h-3 w-3" />
					订阅
				</Button>
			{/if}
		</div>
	</CardContent>
</Card>

<!-- 订阅对话框 -->
<SubscriptionDialog bind:open={dialogOpen} {item} {type} onSuccess={handleSubscriptionSuccess} />
