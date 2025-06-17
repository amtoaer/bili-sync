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
	export let type: 'favorite' | 'collection' | 'upper' = 'favorite';
	export let onSubscriptionSuccess: (() => void) | null = null;

	let dialogOpen = false;

	function getIcon() {
		switch (type) {
			case 'favorite':
				return HeartIcon;
			case 'collection':
				return FolderIcon;
			case 'upper':
				return UserIcon;
			default:
				return VideoIcon;
		}
	}

	function getTypeLabel() {
		switch (type) {
			case 'favorite':
				return '收藏夹';
			case 'collection':
				return '合集';
			case 'upper':
				return 'UP主';
			default:
				return '';
		}
	}

	function getTitle(): string {
		switch (type) {
			case 'favorite':
				return (item as FavoriteWithSubscriptionStatus).title;
			case 'collection':
				return (item as CollectionWithSubscriptionStatus).title;
			case 'upper':
				return (item as UpperWithSubscriptionStatus).uname;
			default:
				return '';
		}
	}

	function getSubtitle(): string {
		switch (type) {
			case 'favorite':
				return `UP主ID: ${(item as FavoriteWithSubscriptionStatus).mid}`;
			case 'collection':
				return `UP主ID: ${(item as CollectionWithSubscriptionStatus).mid}`;
			case 'upper':
				return ''; // UP主不需要副标题
			default:
				return '';
		}
	}

	function getDescription(): string {
		switch (type) {
			case 'upper':
				return (item as UpperWithSubscriptionStatus).sign || '';
			default:
				return '';
		}
	}

	function isDisabled(): boolean {
		switch (type) {
			case 'collection':
				return (item as CollectionWithSubscriptionStatus).invalid;
			case 'upper': {
				return (item as UpperWithSubscriptionStatus).invalid;
			}
			default:
				return false;
		}
	}

	function getDisabledReason(): string {
		switch (type) {
			case 'collection':
				return '已失效';
			case 'upper':
				return '账号已注销';
			default:
				return '';
		}
	}

	function getCount(): number | null {
		switch (type) {
			case 'favorite':
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
			case 'upper':
				return `/image-proxy?url=${(item as UpperWithSubscriptionStatus).face}`;
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

<Card class="group transition-shadow hover:shadow-md {disabled ? 'opacity-60 grayscale' : ''}">
	<CardHeader class="pb-3">
		<div class="flex items-start justify-between gap-3">
			<div class="flex min-w-0 flex-1 items-start gap-3">
				<!-- 头像或图标 -->
				<div
					class="bg-muted flex h-12 w-12 shrink-0 items-center justify-center rounded-lg {disabled
						? 'opacity-50'
						: ''}"
				>
					{#if avatarUrl && type === 'upper'}
						<img
							src={avatarUrl}
							alt={title}
							class="h-full w-full rounded-lg object-cover {disabled ? 'grayscale' : ''}"
							loading="lazy"
						/>
					{:else}
						<Icon class="text-muted-foreground h-6 w-6" />
					{/if}
				</div>

				<!-- 标题和信息 -->
				<div class="min-w-0 flex-1">
					<CardTitle
						class="line-clamp-2 text-base leading-tight {disabled
							? 'text-muted-foreground line-through'
							: ''}"
						{title}
					>
						{title}
					</CardTitle>
					{#if subtitle}
						<div class="text-muted-foreground mt-1 flex items-center gap-1.5 text-sm">
							<UserIcon class="h-3 w-3 shrink-0" />
							<span class="truncate" title={subtitle}>{subtitle}</span>
						</div>
					{/if}
					{#if description}
						<p class="text-muted-foreground mt-1 line-clamp-2 text-xs" title={description}>
							{description}
						</p>
					{/if}
				</div>
			</div>

			<!-- 状态标记 -->
			<div class="flex shrink-0 flex-col items-end gap-2">
				{#if disabled}
					<Badge variant="destructive" class="text-xs">不可用</Badge>
					<div class="text-muted-foreground text-xs">
						{disabledReason}
					</div>
				{:else}
					<Badge variant={subscribed ? 'default' : 'outline'} class="text-xs">
						{subscribed ? '已订阅' : typeLabel}
					</Badge>
					{#if count !== null}
						<div class="text-muted-foreground text-xs">
							{count}
							{countLabel}
						</div>
					{/if}
				{/if}
			</div>
		</div>
	</CardHeader>

	<CardContent class="pt-0">
		<div class="flex justify-end">
			{#if disabled}
				<Button size="sm" variant="outline" disabled class="cursor-not-allowed opacity-50">
					<XIcon class="mr-2 h-4 w-4" />
					不可用
				</Button>
			{:else if subscribed}
				<Button size="sm" variant="outline" disabled class="cursor-not-allowed">
					<CheckIcon class="mr-2 h-4 w-4" />
					已订阅
				</Button>
			{:else}
				<Button
					size="sm"
					variant="default"
					onclick={handleSubscribe}
					class="cursor-pointer"
					{disabled}
				>
					<PlusIcon class="mr-2 h-4 w-4" />
					快捷订阅
				</Button>
			{/if}
		</div>
	</CardContent>
</Card>

<!-- 订阅对话框 -->
<SubscriptionDialog bind:open={dialogOpen} {item} {type} onSuccess={handleSubscriptionSuccess} />
