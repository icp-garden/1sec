import { get, writable } from 'svelte/store';
import { Toast } from './toast';
import { User } from './user/user';
import { BridgeSettingsStore } from './oneSec/settings';
import { Bridge } from './oneSec/bridge';
import type { Chain, Token } from './oneSec/types';

export const guard = createGuardStore();
export const toasts = createToasts();

export const showingModalDialog = createDynamicStore<boolean>(false);

export const user = createDynamicStore<User>(new User());
export const bridge = createDynamicStore<Bridge | undefined>(undefined);
export const bridgeSettings = createDynamicStore<BridgeSettingsStore>(new BridgeSettingsStore());

export function createDynamicStore<T>(resetValue: T, initWith?: T) {
	const { subscribe, set, update } = writable<T>(initWith ?? resetValue);

	return {
		subscribe,
		reset: () => {
			set(resetValue);
		},
		set,
		update,
		tick: () => {
			update((x) => x);
		}
	};
}

function createGuardStore() {
	const { update, subscribe } = writable<{ available: boolean; task?: string }>({
		available: true
	});

	return {
		subscribe,
		lock: (task: string) => {
			update((guard) => {
				if (!guard.available) throw new Error('The guard is currently in use.');
				return { available: false, task };
			});
		},
		unlock: (task: string) => {
			update((guard) => {
				if (guard.task && guard.task !== task)
					throw new Error('The guard is processing another task.');
				return { available: true };
			});
		},
		isLocked: () => {
			return !get(guard).available;
		}
	};
}

function createToasts() {
	const { subscribe, set, update } = writable<Toast[]>([]);
	return {
		subscribe,
		add: (toast: Toast) => update((toasts: Toast[]) => [...toasts, toast]),
		remove: (id: number) => update((toasts: Toast[]) => toasts.filter((toast) => toast.id !== id)),
		reset: () => set([])
	};
}

export const prices = createDynamicStore<Map<Token, number>>(new Map());

function createClockStore() {
	const { subscribe, set, update } = writable<number>(0);
	let clockInterval: NodeJS.Timeout | undefined = undefined;
	return {
		subscribe,
		start: () => {
			clearInterval(clockInterval);
			clockInterval = setInterval(() => {
				update((elapsedTime) => {
					elapsedTime += 0.5;
					return elapsedTime;
				});
			}, 500);
		},
		setElapsedTime: (x: number) => {
			set(x);
		},
		pause: () => {
			clearInterval(clockInterval);
		},
		reset: () => {
			clearInterval(clockInterval);
			set(0);
		}
	};
}
export const clock = createClockStore();

export interface RequestInfo {
	chain: Chain;
	token: Token;
	amount: number;
	address: string;
}

export interface BridgeFormInfo {
	dst: RequestInfo;
	src: RequestInfo;
}

export const bridgeRequest = createDynamicStore<BridgeFormInfo | undefined>(undefined);

export const notifyInterval = createDynamicStore<NodeJS.Timeout | undefined>(undefined);

export const showLoginSidebar = writable(false);
