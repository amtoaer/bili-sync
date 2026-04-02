import { toast } from 'svelte-sonner';
import api from './api';
import type { SysInfo, TaskStatus } from './types';

// 支持的事件类型
export enum EventType {
	Logs = 'logs',
	Tasks = 'tasks',
	SysInfo = 'sysInfo'
}

// 服务器事件响应格式
interface ServerEvent {
	logs?: string;
	tasks?: TaskStatus;
	sysInfo?: SysInfo;
}

// 客户端事件请求格式
interface ClientEvent {
	subscribe?: EventType;
	unsubscribe?: EventType;
}

// 回调函数类型定义
type LogsCallback = (data: string) => void;
type TasksCallback = (data: TaskStatus) => void;
type SysInfoCallback = (data: SysInfo) => void;

export class WebSocketManager {
	private static instance: WebSocketManager;
	private socket: WebSocket | null = null;
	private connected = false;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;
	private baseReconnectDelay = 1000;

	private logsSubscribers: Set<LogsCallback> = new Set();
	private tasksSubscribers: Set<TasksCallback> = new Set();
	private sysInfoSubscribers: Set<SysInfoCallback> = new Set();

	private subscribedEvents: Set<EventType> = new Set();
	private connectionPromise: Promise<boolean> | null = null;
	private authCheckPromise: Promise<boolean> | null = null;
	private authVerified = false;
	private intentionallyClosing = false;

	private constructor() {}

	public static getInstance(): WebSocketManager {
		if (!WebSocketManager.instance) {
			WebSocketManager.instance = new WebSocketManager();
		}
		return WebSocketManager.instance;
	}

	private getAuthToken(): string | null {
		const token = api.getAuthToken()?.trim();
		return token ? token : null;
	}

	private async ensureAuthVerified(): Promise<boolean> {
		const token = this.getAuthToken();
		if (!token) {
			return false;
		}
		if (this.authVerified) {
			return true;
		}
		if (this.authCheckPromise) {
			return this.authCheckPromise;
		}

		this.authCheckPromise = fetch('/api/config', {
			headers: {
				Authorization: token
			}
		})
			.then((response) => {
				this.authVerified = response.ok;
				return response.ok;
			})
			.catch((error) => {
				console.error('Failed to verify WebSocket auth token:', error);
				return false;
			})
			.finally(() => {
				this.authCheckPromise = null;
			});

		return this.authCheckPromise;
	}

	// 连接 WebSocket
	public async connect(): Promise<boolean> {
		if (this.connected) return true;
		if (this.connectionPromise) return this.connectionPromise;
		if (!(await this.ensureAuthVerified())) {
			return false;
		}

		const token = this.getAuthToken();
		if (!token) {
			return false;
		}

		this.connectionPromise = new Promise((resolve) => {
			try {
				const protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
				this.intentionallyClosing = false;
				// 使用 base64URL no padding 编码 token 以避免特殊字符问题
				this.socket = new WebSocket(
					`${protocol}${window.location.host}/api/ws`,
					btoa(token).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
				);
				this.socket.onopen = () => {
					this.connected = true;
					this.reconnectAttempts = 0;
					this.authVerified = true;
					this.connectionPromise = null;
					void this.resubscribeEvents();
					resolve(true);
				};

				this.socket.onmessage = this.handleMessage.bind(this);

				this.socket.onclose = () => {
					this.connected = false;
					this.socket = null;
					this.connectionPromise = null;
					if (this.intentionallyClosing) {
						this.intentionallyClosing = false;
						resolve(false);
						return;
					}
					this.scheduleReconnect();
					resolve(false);
				};

				this.socket.onerror = (error) => {
					console.error('WebSocket error:', error);
					toast.error('WebSocket 连接发生错误，请检查网络或稍后重试');
				};
			} catch (error) {
				this.connectionPromise = null;
				resolve(false);
				console.error('Failed to create WebSocket:', error);
				toast.error('创建 WebSocket 连接失败，请检查网络或稍后重试');
				this.scheduleReconnect();
			}
		});

		return this.connectionPromise;
	}

	private handleMessage(event: MessageEvent): void {
		try {
			const data = JSON.parse(event.data) as ServerEvent;

			if (data.logs !== undefined) {
				this.notifyLogsSubscribers(data.logs);
			} else if (data.tasks !== undefined) {
				this.notifyTasksSubscribers(data.tasks);
			} else if (data.sysInfo !== undefined) {
				this.notifySysInfoSubscribers(data.sysInfo);
			}
		} catch (error) {
			console.error('Failed to parse WebSocket message:', error, event.data);
			toast.error('解析 WebSocket 消息失败', {
				description: `消息内容：${event.data}\n错误信息：${error instanceof Error ? error.message : String(error)}`
			});
		}
	}

	private async sendMessage(message: ClientEvent): Promise<void> {
		if (!this.connected) {
			const connected = await this.connect();
			if (!connected) {
				return;
			}
		}

		try {
			this.socket!.send(JSON.stringify(message));
		} catch (error) {
			console.error('Failed to send message:', error);
			toast.error('发送 WebSocket 消息失败', {
				description: `消息内容：${JSON.stringify(message)}\n错误信息：${error instanceof Error ? error.message : String(error)}`
			});
		}
	}

	private async subscribe(eventType: EventType): Promise<void> {
		if (this.subscribedEvents.has(eventType)) return;

		this.subscribedEvents.add(eventType);
		await this.sendMessage({ subscribe: eventType });
	}

	// 取消订阅事件
	private async unsubscribe(eventType: EventType): Promise<void> {
		if (!this.subscribedEvents.has(eventType)) return;

		this.subscribedEvents.delete(eventType);
		if (!this.connected) {
			return;
		}
		await this.sendMessage({ unsubscribe: eventType });
	}

	private async resubscribeEvents(): Promise<void> {
		await Promise.all(
			Array.from(this.subscribedEvents).map(async (eventType) => {
				await this.sendMessage({ subscribe: eventType });
			})
		);
	}

	private scheduleReconnect(): void {
		if (!this.getAuthToken()) {
			return;
		}

		if (this.reconnectTimer !== null) {
			clearTimeout(this.reconnectTimer);
		}

		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			console.log('Max reconnect attempts reached');
			return;
		}

		const delay = this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts);
		console.log(`Scheduling reconnect in ${delay}ms`);

		this.reconnectTimer = setTimeout(() => {
			this.reconnectAttempts++;
			void this.connect();
		}, delay);
	}

	public markAuthVerified(): void {
		this.authVerified = true;
		if (!this.connected && this.subscribedEvents.size > 0) {
			void this.connect();
		}
	}

	public markAuthInvalid(): void {
		this.authVerified = false;
		this.authCheckPromise = null;
		this.disconnect(false);
	}

	public onAuthTokenChanged(): void {
		this.authVerified = false;
		this.authCheckPromise = null;
		this.disconnect(false);
	}

	public subscribeToLogs(callback: LogsCallback): () => void {
		this.logsSubscribers.add(callback);

		if (this.logsSubscribers.size === 1) {
			this.subscribe(EventType.Logs);
		}

		return () => {
			this.logsSubscribers.delete(callback);
			if (this.logsSubscribers.size === 0) {
				this.unsubscribe(EventType.Logs);
			}
		};
	}

	// 订阅任务状态
	public subscribeToTasks(callback: TasksCallback): () => void {
		this.tasksSubscribers.add(callback);

		if (this.tasksSubscribers.size === 1) {
			this.subscribe(EventType.Tasks);
		}

		return () => {
			this.tasksSubscribers.delete(callback);
			if (this.tasksSubscribers.size === 0) {
				this.unsubscribe(EventType.Tasks);
			}
		};
	}

	public subscribeToSysInfo(callback: SysInfoCallback): () => void {
		this.sysInfoSubscribers.add(callback);

		if (this.sysInfoSubscribers.size === 1) {
			this.subscribe(EventType.SysInfo);
		}

		return () => {
			this.sysInfoSubscribers.delete(callback);
			if (this.sysInfoSubscribers.size === 0) {
				this.unsubscribe(EventType.SysInfo);
			}
		};
	}

	private notifyLogsSubscribers(data: string): void {
		this.logsSubscribers.forEach((callback) => {
			try {
				callback(data);
			} catch (error) {
				console.error('Error in logs subscriber callback:', error);
			}
		});
	}

	private notifyTasksSubscribers(data: TaskStatus): void {
		this.tasksSubscribers.forEach((callback) => {
			try {
				callback(data);
			} catch (error) {
				console.error('Error in tasks subscriber callback:', error);
			}
		});
	}

	private notifySysInfoSubscribers(data: SysInfo): void {
		this.sysInfoSubscribers.forEach((callback) => {
			try {
				callback(data);
			} catch (error) {
				console.error('Error in sysInfo subscriber callback:', error);
			}
		});
	}

	public disconnect(clearSubscriptions: boolean = true): void {
		if (this.socket) {
			this.intentionallyClosing = true;
			this.socket.close();
			this.socket = null;
		} else {
			this.intentionallyClosing = false;
		}

		if (this.reconnectTimer !== null) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		this.connected = false;
		this.connectionPromise = null;
		if (clearSubscriptions) {
			this.subscribedEvents.clear();
		}
	}
}

export const wsManager = WebSocketManager.getInstance();
