<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { toast } from 'svelte-sonner';
	import api from '$lib/api';
	import type { Credential, ApiError } from '$lib/types';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import CheckCircle from '@lucide/svelte/icons/check-circle';
	import LoaderCircle from '@lucide/svelte/icons/loader-circle';
	import QRCode from 'qrcode';

	export let onSuccess: (credential: Credential) => void;

	type Status = 'idle' | 'loading' | 'showing' | 'success' | 'expired' | 'error';

	let status: Status = 'idle';
	let qrcodeUrl = '';
	let qrcodeKey = '';
	let qrcodeDataUrl = ''; // 存储生成的二维码图片 Data URL
	let countdown = 180;
	let pollInterval: ReturnType<typeof setInterval> | null = null;
	let countdownInterval: ReturnType<typeof setInterval> | null = null;
	let scanned = false;
	let errorMessage = '';
	let isPolling = false; // 添加轮询标志

	async function generateQrcode() {
		// 先停止之前的轮询和倒计时（排他性）
		stopPolling();
		stopCountdown();

		status = 'loading';
		errorMessage = '';
		scanned = false;

		try {
			const response = await api.generateQrcode();
			qrcodeUrl = response.data.url;
			qrcodeKey = response.data.qrcode_key;
			countdown = 180;

			// 将 URL 转换为二维码图片
			qrcodeDataUrl = await QRCode.toDataURL(qrcodeUrl, {
				width: 256,
				margin: 2,
				color: {
					dark: '#000000',
					light: '#FFFFFF'
				}
			});

			status = 'showing';

			// 开始轮询和倒计时
			startPolling();
			startCountdown();
		} catch (error) {
			console.error('生成二维码失败:', error);
			status = 'error';
			errorMessage = (error as ApiError).message || '生成二维码失败';
			toast.error('生成二维码失败', {
				description: (error as ApiError).message
			});
		}
	}

	async function pollStatus() {
		// 如果已经停止轮询，直接返回
		if (!qrcodeKey || !isPolling) return;

		try {
			const response = await api.pollQrcode(qrcodeKey);
			const pollResult = response.data;

			// 再次检查是否还在轮询（防止在请求过程中状态改变）
			if (!isPolling) return;

			if (pollResult.status === 'success') {
				stopPolling();
				stopCountdown();
				status = 'success';
				toast.success('登录成功！');
				onSuccess(pollResult.credential);
			} else if (pollResult.status === 'pending') {
				scanned = pollResult.scanned || false;
			} else if (pollResult.status === 'expired') {
				stopPolling();
				stopCountdown();
				status = 'expired';
				toast.error('二维码已过期');
			}
		} catch (error) {
			console.error('轮询登录状态失败:', error);
			// 检查是否还在轮询，如果不在则不继续重试
			if (!isPolling) return;
		}
	}

	function startPolling() {
		isPolling = true;
		pollInterval = setInterval(pollStatus, 2000);
	}

	function stopPolling() {
		isPolling = false; // 立即设置标志为 false
		if (pollInterval) {
			clearInterval(pollInterval);
			pollInterval = null;
		}
	}

	function startCountdown() {
		countdownInterval = setInterval(() => {
			countdown--;
			if (countdown <= 0) {
				stopPolling();
				stopCountdown();
				status = 'expired';
				toast.error('二维码已过期');
			}
		}, 1000);
	}

	function stopCountdown() {
		if (countdownInterval) {
			clearInterval(countdownInterval);
			countdownInterval = null;
		}
	}

	onDestroy(() => {
		stopPolling();
		stopCountdown();
	});
</script>

<div class="qr-login-container">
	{#if status === 'idle' || status === 'error'}
		<div class="flex flex-col items-center gap-3">
			{#if status === 'error'}
				<p class="text-sm text-destructive">{errorMessage}</p>
			{/if}
			<Button onclick={generateQrcode} class="w-full">
				<RefreshCw class="mr-2 h-4 w-4" />
				{status === 'error' ? '重新获取二维码' : '扫码登录'}
			</Button>
		</div>
	{:else if status === 'loading'}
		<div class="flex flex-col items-center gap-3">
			<LoaderCircle class="h-8 w-8 animate-spin text-muted-foreground" />
			<p class="text-sm text-muted-foreground">正在生成二维码...</p>
		</div>
	{:else if status === 'showing'}
		<Card.Root class="border-0 shadow-none">
			<Card.Content class="p-4">
				<div class="flex flex-col items-center gap-4">
					<div class="rounded-lg border-2 border-border p-3 bg-white">
						<!-- 使用生成的二维码图片 Data URL -->
						<img src={qrcodeDataUrl} alt="登录二维码" class="w-48 h-48" />
					</div>

					<div class="text-center space-y-2">
						{#if scanned}
							<div class="flex items-center justify-center gap-2 text-primary">
								<LoaderCircle class="h-5 w-5 animate-spin" />
								<p class="font-medium">已扫描，请在手机上确认登录</p>
							</div>
						{:else}
							<p class="text-sm text-muted-foreground">请使用哔哩哔哩 APP 扫描二维码</p>
						{/if}

						<div class="flex items-center justify-center gap-2">
							<span class="text-xs text-muted-foreground">有效时间:</span>
							<span
								class="text-sm font-mono font-bold"
								class:text-destructive={countdown < 30}
								class:text-primary={countdown >= 30}
							>
								{Math.floor(countdown / 60)}:{String(countdown % 60).padStart(2, '0')}
							</span>
						</div>
					</div>

					<Button variant="outline" size="sm" onclick={generateQrcode} class="w-full">
						<RefreshCw class="mr-2 h-4 w-4" />
						刷新二维码
					</Button>
				</div>
			</Card.Content>
		</Card.Root>
	{:else if status === 'success'}
		<div class="flex flex-col items-center gap-3 py-4">
			<CheckCircle class="h-12 w-12 text-green-500" />
			<p class="text-lg font-medium text-green-600">登录成功！</p>
			<p class="text-sm text-muted-foreground">凭证已自动保存</p>
		</div>
	{:else if status === 'expired'}
		<div class="flex flex-col items-center gap-3">
			<p class="text-sm text-muted-foreground">二维码已过期</p>
			<Button onclick={generateQrcode} variant="outline" class="w-full">
				<RefreshCw class="mr-2 h-4 w-4" />
				重新获取
			</Button>
		</div>
	{/if}
</div>

<style>
	.qr-login-container {
		width: 100%;
	}
</style>
