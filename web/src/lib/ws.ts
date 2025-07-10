import { toast } from 'svelte-sonner';
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

	private constructor() {}

	public static getInstance(): WebSocketManager {
		if (!WebSocketManager.instance) {
			WebSocketManager.instance = new WebSocketManager();
		}
		return WebSocketManager.instance;
	}

	// 连接 WebSocket
	public connect(): void {
		if (this.connected || this.connecting) return;

		this.connecting = true;
		const token = localStorage.getItem('authToken') || '';

		try {
			this.socket = new WebSocket(`ws://${window.location.host}/api/ws`, [token]);

			this.socket.onopen = () => {
				this.connected = true;
				this.connecting = false;
				this.reconnectAttempts = 0;

				this.resubscribeEvents();
			};

			this.socket.onmessage = this.handleMessage.bind(this);

			this.socket.onclose = () => {
				this.connected = false;
				this.connecting = false;
				this.scheduleReconnect();
			};

			this.socket.onerror = (error) => {
				console.error('WebSocket error:', error);
				toast.error('WebSocket 连接发生错误，请检查网络或稍后重试');
			};
		} catch (error) {
			this.connecting = false;
			console.error('Failed to create WebSocket:', error);
			toast.error('创建 WebSocket 连接失败，请检查网络或稍后重试');
			this.scheduleReconnect();
		}
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
				description: `消息内容: ${event.data}\n错误信息: ${error instanceof Error ? error.message : String(error)}`
			});
		}
	}

	private sendMessage(message: ClientEvent): void {
		if (!this.connected || !this.socket) {
			console.warn('Cannot send message: WebSocket not connected');
			return;
		}

		try {
			this.socket.send(JSON.stringify(message));
		} catch (error) {
			console.error('Failed to send message:', error);
			toast.error('发送 WebSocket 消息失败', {
				description: `消息内容: ${JSON.stringify(message)}\n错误信息: ${error instanceof Error ? error.message : String(error)}`
			});
		}
	}

	private subscribe(eventType: EventType): void {
		if (this.subscribedEvents.has(eventType)) return;

		this.sendMessage({ subscribe: eventType });
		this.subscribedEvents.add(eventType);
	}

	// 取消订阅事件
	private unsubscribe(eventType: EventType): void {
		if (!this.subscribedEvents.has(eventType)) return;

		this.sendMessage({ unsubscribe: eventType });
		this.subscribedEvents.delete(eventType);
	}

	private resubscribeEvents(): void {
		for (const eventType of this.subscribedEvents) {
			this.sendMessage({ subscribe: eventType });
		}
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
		this.connect();
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
		this.connect();
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
		this.connect();
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
		this.subscribedEvents.clear();
	}
}

export const wsManager = WebSocketManager.getInstance();
