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
type ErrorCallback = (error: Event) => void;

export class WebSocketManager {
	private static instance: WebSocketManager;
	private socket: WebSocket | null = null;
	private connected = false;
	private connecting = false;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;
	private baseReconnectDelay = 1000;

	private logsSubscribers: Set<LogsCallback> = new Set();
	private tasksSubscribers: Set<TasksCallback> = new Set();
	private sysInfoSubscribers: Set<SysInfoCallback> = new Set();
	private errorSubscribers: Set<ErrorCallback> = new Set();

	private subscribedEvents: Set<EventType> = new Set();
	private connectionPromise: Promise<void> | null = null;

	private constructor() {}

	public static getInstance(): WebSocketManager {
		if (!WebSocketManager.instance) {
			WebSocketManager.instance = new WebSocketManager();
		}
		return WebSocketManager.instance;
	}

	// 连接 WebSocket
	public connect(): Promise<void> {
		if (this.connected) return Promise.resolve();
		if (this.connectionPromise) return this.connectionPromise;

		this.connectionPromise = new Promise((resolve, reject) => {
			this.connecting = true;
			const token = api.getAuthToken() || '';

			try {
				const protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
				// 使用 base64URL no padding 编码 token 以避免特殊字符问题
				this.socket = new WebSocket(
					`${protocol}${window.location.host}/api/ws`,
					btoa(token).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
				);
				this.socket.onopen = () => {
					this.connected = true;
					this.connecting = false;
					this.reconnectAttempts = 0;
					this.connectionPromise = null;
					this.resubscribeEvents();
					resolve();
				};

				this.socket.onmessage = this.handleMessage.bind(this);

				this.socket.onclose = () => {
					this.connected = false;
					this.connecting = false;
					this.connectionPromise = null;
					this.scheduleReconnect();
				};

				this.socket.onerror = (error) => {
					console.error('WebSocket error:', error);
					this.connecting = false;
					this.connectionPromise = null;
					reject(error);
					toast.error('WebSocket 连接发生错误，请检查网络或稍后重试');
				};
			} catch (error) {
				this.connecting = false;
				this.connectionPromise = null;
				reject(error);
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
			await this.connect();
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

		await this.sendMessage({ subscribe: eventType });
		this.subscribedEvents.add(eventType);
	}

	// 取消订阅事件
	private async unsubscribe(eventType: EventType): Promise<void> {
		if (!this.subscribedEvents.has(eventType)) return;

		await this.sendMessage({ unsubscribe: eventType });
		this.subscribedEvents.delete(eventType);
	}

	private async resubscribeEvents(): Promise<void> {
		await Promise.all(
			Array.from(this.subscribedEvents).map(async (eventType) => {
				await this.sendMessage({ subscribe: eventType });
			})
		);
	}

	private scheduleReconnect(): void {
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
			this.connect();
		}, delay);
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

	public disconnect(): void {
		if (this.socket) {
			this.socket.close();
			this.socket = null;
		}

		if (this.reconnectTimer !== null) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		this.connected = false;
		this.connecting = false;
		this.connectionPromise = null;
		this.subscribedEvents.clear();
	}
}

export const wsManager = WebSocketManager.getInstance();
