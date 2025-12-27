import { type Snippet } from 'svelte';

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

export interface NotificationOptions {
    id?: string;
    message?: string;
    component?: Snippet;
    type?: NotificationType;
    duration?: number;
    dismissible?: boolean;
}

export interface NotificationItem extends NotificationOptions {
    id: string;
    type: NotificationType;
    duration: number;
    dismissible: boolean;
    createdAt: number;
}

class NotificationStore {
    notifications = $state<NotificationItem[]>([]);
    private defaultDuration = 5000;

    add(options: NotificationOptions) {
        const id = options.id || Math.random().toString(36).substring(2, 9);
        const notification: NotificationItem = {
            id,
            type: options.type || 'info',
            duration: options.duration ?? this.defaultDuration,
            dismissible: options.dismissible ?? true,
            createdAt: Date.now(),
            ...options
        };

        this.notifications.push(notification);

        if (notification.duration > 0) {
            setTimeout(() => {
                this.dismiss(id);
            }, notification.duration);
        }

        return id;
    }

    success(message: string, options?: Omit<NotificationOptions, 'message' | 'type'>) {
        return this.add({ ...options, message, type: 'success' });
    }

    error(message: string, options?: Omit<NotificationOptions, 'message' | 'type'>) {
        return this.add({ ...options, message, type: 'error', duration: 8000 }); // Errors last longer by default
    }

    warning(message: string, options?: Omit<NotificationOptions, 'message' | 'type'>) {
        return this.add({ ...options, message, type: 'warning' });
    }

    info(message: string, options?: Omit<NotificationOptions, 'message' | 'type'>) {
        return this.add({ ...options, message, type: 'info' });
    }

    custom(component: Snippet, options?: Omit<NotificationOptions, 'component'>) {
        return this.add({ ...options, component });
    }

    dismiss(id: string) {
        this.notifications = this.notifications.filter(n => n.id !== id);
    }

    clear() {
        this.notifications = [];
    }
}

export const notifications = new NotificationStore();
