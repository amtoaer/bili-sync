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

	/**
	 * 扫码登录组件
	 *
	 * 状态流转:
	 * idle -> loading -> showing -> (success | expired | error)
	 *
	 * @prop onSuccess - 登录成功回调，接收完整的凭证对象
	 */

	// 常量配置
	const QR_EXPIRE_TIME = 180; // 二维码有效期（秒）
	const POLL_INTERVAL = 2000; // 轮询间隔（毫秒）
	const COUNTDOWN_INTERVAL = 1000; // 倒计时更新间隔（毫秒）
	const QR_SIZE = 256; // 二维码图片尺寸（像素）
	const QR_MARGIN = 2; // 二维码边距
	const WARNING_TIME = 30; // 显示警告颜色的剩余时间（秒）

	export let onSuccess: (credential: Credential) => void;

	type Status = 'idle' | 'loading' | 'showing' | 'success' | 'expired' | 'error';

	let status: Status = 'idle';
	let qrcodeUrl = ''; // B站返回的二维码 URL（需要转换为图片）
	let qrcodeKey = ''; // 用于轮询的认证 token
	let qrcodeDataUrl = ''; // 生成的二维码图片 Data URL
	let countdown = QR_EXPIRE_TIME; // 倒计时
	let pollInterval: ReturnType<typeof setInterval> | null = null;
	let countdownInterval: ReturnType<typeof setInterval> | null = null;
	let scanned = false; // 是否已扫描
	let errorMessage = '';
	let isPolling = false; // 轮询标志，确保轮询排他性

	/**
	 * 生成二维码
	 *
	 * 1. 停止之前的轮询和倒计时（确保排他性）
	 * 2. 调用后端 API 获取二维码信息
	 * 3. 将 URL 转换为二维码图片
	 * 4. 开始轮询登录状态
	 */
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
			countdown = QR_EXPIRE_TIME;

			// 将 URL 转换为二维码图片
			qrcodeDataUrl = await QRCode.toDataURL(qrcodeUrl, {
				width: QR_SIZE,
				margin: QR_MARGIN,
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

	/**
	 * 轮询登录状态
	 *
	 * 每次调用前检查 isPolling 标志，确保轮询排他性。
	 * 异步请求后再次检查，防止在请求过程中状态已改变。
	 */
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

	/**
	 * 启动轮询
	 *
	 * 设置轮询标志并启动定时器
	 */
	function startPolling() {
		isPolling = true;
		pollInterval = setInterval(pollStatus, POLL_INTERVAL);
	}

	/**
	 * 停止轮询
	 *
	 * 立即设置轮询标志为 false，清除定时器
	 */
	function stopPolling() {
		isPolling = false; // 立即设置标志为 false
		if (pollInterval) {
			clearInterval(pollInterval);
			pollInterval = null;
		}
	}

	/**
	 * 启动倒计时
	 *
	 * 每秒减少倒计时，到期后自动停止轮询并标记为过期
	 */
	function startCountdown() {
		countdownInterval = setInterval(() => {
			countdown--;
			if (countdown <= 0) {
				stopPolling();
				stopCountdown();
				status = 'expired';
				toast.error('二维码已过期');
			}
		}, COUNTDOWN_INTERVAL);
	}

	/**
	 * 停止倒计时
	 *
	 * 清除倒计时定时器
	 */
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
				<p class="text-destructive text-sm">{errorMessage}</p>
			{/if}
			<Button onclick={generateQrcode} class="w-full">
				<RefreshCw class="mr-2 h-4 w-4" />
				{status === 'error' ? '重新获取二维码' : '扫码登录'}
			</Button>
		</div>
	{:else if status === 'loading'}
		<div class="flex flex-col items-center gap-3">
			<LoaderCircle class="text-muted-foreground h-8 w-8 animate-spin" />
			<p class="text-muted-foreground text-sm">正在生成二维码...</p>
		</div>
	{:else if status === 'showing'}
		<Card.Root class="border-0 shadow-none">
			<Card.Content class="p-4">
				<div class="flex flex-col items-center gap-4">
					<div class="border-border rounded-lg border-2 bg-white p-3">
						<!-- 使用生成的二维码图片 Data URL -->
						<img src={qrcodeDataUrl} alt="登录二维码" class="h-48 w-48" />
					</div>

					<div class="space-y-2 text-center">
						{#if scanned}
							<div class="text-primary flex items-center justify-center gap-2">
								<LoaderCircle class="h-5 w-5 animate-spin" />
								<p class="font-medium">已扫描，请在手机上确认登录</p>
							</div>
						{:else}
							<p class="text-muted-foreground text-sm">请使用哔哩哔哩 APP 扫描二维码</p>
						{/if}

						<div class="flex items-center justify-center gap-2">
							<span class="text-muted-foreground text-xs">有效时间:</span>
							<span
								class="font-mono text-sm font-bold"
								class:text-destructive={countdown < WARNING_TIME}
								class:text-primary={countdown >= WARNING_TIME}
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
			<p class="text-muted-foreground text-sm">凭证已自动保存</p>
		</div>
	{:else if status === 'expired'}
		<div class="flex flex-col items-center gap-3">
			<p class="text-muted-foreground text-sm">二维码已过期</p>
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
